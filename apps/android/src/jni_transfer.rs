//! JNI entry points for the system pickers and the share sheet (`dev.openpaint.Native`). The
//! Kotlin host takes the runtime's requests as lines after each frame and hands back what the
//! user chose as a file descriptor it detached; files to share are made here, in the app's cache,
//! and a video's encoder is lent at once (`video.rs`). All calls arrive on the Android main
//! thread; errors are logged, nothing is thrown into Java.

use std::fs::File;
use std::os::fd::{FromRawFd, OwnedFd};
use std::sync::Arc;

use jni::errors::LogErrorAndDefault;
use jni::objects::{JClass, JObject, JString};
use jni::sys::{jint, jlong};
use jni::{Env, EnvUnowned};
use openpaint_runtime::{HostAnswer, HostRequest, PortError};

use crate::error::HostError;
use crate::jni_entry::{Host, host};
use crate::transfer::{FileSink, FileSource, Pick, request_line, share_file};
use crate::video::{AndroidVideo, VideoClass};

/// The next request for the Kotlin host as a line ("save\t<mime>\t<name>", "open",
/// "send\t<mime>\t<path>", "send-video\t<name>"), or null when there is none. Files to share and
/// video encoders are made here first.
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_openpaint_Native_request<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    handle: jlong,
) -> JObject<'local> {
    env.with_env(|env| -> Result<JObject<'local>, HostError> {
        let Some(host) = host(handle) else {
            return Ok(JObject::null());
        };
        while let Some(request) = host.runtime.host_request() {
            log::info!("asked of the system: {request:?}");
            match request_line(&request, &host.share_dir) {
                Some((line, pick)) => {
                    if pick.is_some() {
                        host.pending = pick;
                    }
                    return Ok(env.new_string(line)?.into());
                }
                None => serve(env, host, &request),
            }
        }
        Ok(JObject::null())
    })
    .resolve::<LogErrorAndDefault>()
}

/// Answers at once what the Rust side serves itself: the file a `ShareFile` request asks for, the
/// encoder a `Video` request asks for.
fn serve(env: &mut Env<'_>, host: &mut Host, request: &HostRequest) {
    let answer = match request {
        HostRequest::ShareFile { name, .. } => match share_file(&host.share_dir, name) {
            Ok((path, file)) => {
                log::info!("sharing through {}", path.display());
                HostAnswer::File(Box::new(FileSink(file)))
            }
            Err(error) => HostAnswer::Failed(error),
        },
        HostRequest::Video { .. } => match video_class(env, host) {
            Ok(class) => {
                let video = AndroidVideo::new(class, host.share_dir.clone());
                HostAnswer::Video(Box::new(video))
            }
            Err(error) => HostAnswer::Failed(PortError::Device {
                detail: format!("no video encoder: {error}"),
            }),
        },
        _ => return,
    };
    host.runtime.answer(answer);
}

/// The Kotlin encoder's class, found on this (the main) thread the first time a video is made.
fn video_class(env: &mut Env<'_>, host: &mut Host) -> Result<Arc<VideoClass>, jni::errors::Error> {
    if let Some(class) = &host.video {
        return Ok(Arc::clone(class));
    }
    let class = Arc::new(VideoClass::find(env)?);
    host.video = Some(Arc::clone(&class));
    Ok(class)
}

/// What the user chose: `fd` is a descriptor the Kotlin host detached for us (owned from now
/// on), -1 when the picker was cancelled; `error` is set when the system failed.
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_openpaint_Native_answer<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    handle: jlong,
    fd: jint,
    error: JString<'local>,
) {
    env.with_env(|env| -> Result<(), HostError> {
        let failure = if error.is_null() {
            None
        } else {
            Some(error.try_to_string(env)?)
        };
        let file = (fd >= 0).then(|| {
            // SAFETY: Kotlin detached this descriptor from its ParcelFileDescriptor and passes
            // its ownership here; nothing else closes it.
            File::from(unsafe { OwnedFd::from_raw_fd(fd) })
        });
        let Some(host) = host(handle) else {
            return Ok(());
        };
        let answer = match (failure, file, host.pending.take()) {
            (Some(detail), _, _) => HostAnswer::Failed(PortError::Device { detail }),
            (None, Some(file), Some(Pick::Save)) => HostAnswer::File(Box::new(FileSink(file))),
            (None, Some(file), Some(Pick::Open)) => HostAnswer::Image(Box::new(FileSource(file))),
            (None, Some(_), None) => {
                log::warn!("a file came back for nothing asked");
                return Ok(());
            }
            (None, None, _) => HostAnswer::Cancelled,
        };
        host.runtime.answer(answer);
        Ok(())
    })
    .resolve::<LogErrorAndDefault>();
}
