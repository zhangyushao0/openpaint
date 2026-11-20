//! The video encoder the platform lends the replay (S163, `VideoEncoder`): Kotlin's
//! `dev.openpaint.VideoEncoder` (MediaCodec H.264 through its input surface, MediaMuxer into an
//! MP4 in the album's Movies/OpenPaint or the share directory), driven over JNI from the video
//! worker's thread. The class is found once on the main thread (`VideoClass`: FindClass off it
//! sees only the system's classes) and kept as a global reference with the VM; the worker
//! attaches itself to the VM for its calls. A frame goes over as a direct ByteBuffer on the
//! worker's own bytes, read during the call only. The Kotlin side answers each call with null, or
//! "encoder\t<why>" or "write\t<why>"; an exception it throws counts as the encoder's.

use std::path::PathBuf;
use std::sync::Arc;

use jni::errors::Error;
use jni::objects::{Global, JClass, JObject, JString, JValue};
use jni::sys::{jint, jlong};
use jni::{Env, JavaVM, jni_sig, jni_str};
use openpaint_app::{Delivery, PortError, VideoEncoder, VideoError, VideoOrder};

use crate::transfer::share_file;

/// The Kotlin encoder's class and the VM to reach it from any thread.
pub(crate) struct VideoClass {
    vm: JavaVM,
    class: Global<JClass<'static>>,
}

impl VideoClass {
    /// Finds the class; call on the main thread (a JNI call from Kotlin).
    pub(crate) fn find(env: &mut Env<'_>) -> Result<Self, Error> {
        let class = env.find_class(jni_str!("dev/openpaint/VideoEncoder"))?;
        Ok(Self {
            class: env.new_global_ref(&class)?,
            vm: env.get_java_vm()?,
        })
    }
}

/// One video's encoder: a Kotlin `VideoEncoder` from `begin` until `finish` or `cancel`.
pub(crate) struct AndroidVideo {
    class: Arc<VideoClass>,
    /// Where a video for the share sheet is made (`ShareProvider` serves it).
    share_dir: PathBuf,
    encoder: Option<Global<JObject<'static>>>,
}

impl AndroidVideo {
    pub(crate) const fn new(class: Arc<VideoClass>, share_dir: PathBuf) -> Self {
        Self {
            class,
            share_dir,
            encoder: None,
        }
    }

    /// `call` on the Kotlin encoder from this thread, which reads its answer: the failure it
    /// answered, if any.
    fn call(
        &self,
        call: impl FnOnce(&mut Env<'_>, &JObject<'static>) -> Result<Option<String>, Error>,
    ) -> Result<(), VideoError> {
        let encoder = self
            .encoder
            .as_ref()
            .ok_or_else(|| encoder("no video begun"))?;
        failed(
            self.class
                .vm
                .attach_current_thread(|env| call(env, encoder.as_obj())),
        )
    }
}

impl VideoEncoder for AndroidVideo {
    fn begin(&mut self, order: &VideoOrder) -> Result<(), VideoError> {
        // A video to share is made where the share sheet's files are, the one before it gone.
        let shared = match order.delivery {
            Delivery::Save => None,
            Delivery::Share => Some(
                share_file(&self.share_dir, &order.name)
                    .map_err(VideoError::Write)?
                    .0,
            ),
        };
        let side = |side: u32| jint::try_from(side).unwrap_or(jint::MAX);
        let [width, height] = order.size.map(side);
        let fps = side(order.fps);
        let class = &self.class;
        let begun = class.vm.attach_current_thread(|env| {
            let made = env.new_object(&class.class, jni_sig!("()V"), &[])?;
            let name = env.new_string(&order.name)?;
            let path = match &shared {
                Some(path) => JObject::from(env.new_string(path.display().to_string())?),
                None => JObject::null(),
            };
            let args = [
                JValue::from(&name),
                width.into(),
                height.into(),
                fps.into(),
                JValue::from(&path),
            ];
            let signature = jni_sig!("(Ljava/lang/String;IIILjava/lang/String;)Ljava/lang/String;");
            let failure = env
                .call_method(&made, jni_str!("begin"), signature, &args)?
                .l()?;
            let global = env.new_global_ref(&made)?;
            Ok((global, answered(env, failure)?))
        });
        let (global, failure) = begun.map_err(|error: Error| encoder(&error.to_string()))?;
        self.encoder = Some(global);
        log::info!("video {} at {width} x {height}, {fps} fps", order.name);
        failed(Ok(failure))
    }

    fn frame(&mut self, rgba: &[u8], at_us: u64) -> Result<(), VideoError> {
        let at = jlong::try_from(at_us).unwrap_or(jlong::MAX);
        self.call(|env, encoder| {
            // SAFETY: the bytes outlive the call, and the Kotlin side reads the buffer only during
            // it (uploaded to a texture) and keeps no reference to it, nor writes to it.
            let buffer =
                unsafe { env.new_direct_byte_buffer(rgba.as_ptr().cast_mut(), rgba.len()) }?;
            let signature = jni_sig!("(Ljava/nio/ByteBuffer;J)Ljava/lang/String;");
            let args = [JValue::from(&buffer), at.into()];
            let failure = env
                .call_method(encoder, jni_str!("frame"), signature, &args)?
                .l()?;
            answered(env, failure)
        })
    }

    fn finish(&mut self) -> Result<(), VideoError> {
        let finished = self.call(|env, encoder| {
            let signature = jni_sig!("()Ljava/lang/String;");
            let failure = env
                .call_method(encoder, jni_str!("finish"), signature, &[])?
                .l()?;
            answered(env, failure)
        });
        self.encoder = None;
        finished
    }

    fn cancel(&mut self) {
        let Some(encoder) = self.encoder.take() else {
            return;
        };
        let cancelled = self
            .class
            .vm
            .attach_current_thread(|env| -> Result<(), Error> {
                env.call_method(encoder.as_obj(), jni_str!("cancel"), jni_sig!("()V"), &[])?;
                Ok(())
            });
        if let Err(error) = cancelled {
            log::error!("the video could not be cancelled: {error}");
        }
    }
}

impl Drop for AndroidVideo {
    /// A video neither finished nor cancelled (a worker that stopped) leaves nothing behind.
    fn drop(&mut self) {
        self.cancel();
    }
}

/// The failure the Kotlin side answered: null for none.
fn answered(env: &mut Env<'_>, failure: JObject<'_>) -> Result<Option<String>, Error> {
    if failure.is_null() {
        return Ok(None);
    }
    let failure = env.cast_local::<JString>(failure)?;
    Ok(Some(failure.try_to_string(env)?))
}

/// What a call's answer means for the video: a failure "write\t…" is the file's, any other the
/// encoder's; an error of the call itself the encoder's.
fn failed(answer: Result<Option<String>, Error>) -> Result<(), VideoError> {
    let failure = answer.map_err(|error| encoder(&error.to_string()))?;
    let Some(line) = failure else {
        return Ok(());
    };
    log::error!("video: {line}");
    match line.split_once('\t') {
        Some(("write", why)) => Err(VideoError::Write(device(why))),
        Some((_, why)) => Err(VideoError::Encoder(device(why))),
        None => Err(VideoError::Encoder(device(&line))),
    }
}

fn encoder(why: &str) -> VideoError {
    VideoError::Encoder(device(why))
}

fn device(why: &str) -> PortError {
    PortError::Device {
        detail: why.to_owned(),
    }
}
