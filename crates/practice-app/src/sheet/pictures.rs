//! The lesson's pictures as the store reads them for the screen (a single shape's with its ground
//! faded once, S99), and those laid under an exercise's marks on the sheet's picture layer
//! (`Facts::under`, S112, `layers/picture.rs`): the k-th exercise shown whose kind lays its picture
//! under is the layer's k-th, laid once its picture is read; a pen waits for it.

use std::sync::Arc;

use openpaint_practice::ItemRef;

use super::PracticeSheet;
use crate::layers::Under;
use crate::pictures::{PictureImage, grounded};

impl PracticeSheet {
    /// The next picture of the sheet to read for the screen, once each.
    pub fn picture_request(&mut self) -> Option<ItemRef> {
        let item = (self.sheet.pictures.iter().flatten())
            .map(|picture| &picture.item)
            .find(|item| !self.asked.contains(*item))?
            .clone();
        self.asked.insert(item.clone());
        Some(item)
    }

    /// The store read `item`'s picture (`None`: it could not); one whose ground shows faded (a
    /// single shape's) faded once (S99).
    pub fn picture_read(&mut self, item: ItemRef, image: Option<PictureImage>) {
        self.images.retain(|(kept, _)| *kept != item);
        self.grounded.retain(|(kept, _)| *kept != item);
        let faded = (self.sheet.pictures.iter().zip(&self.sheet.kinds))
            .filter(|(_, kind)| kind.facts().grounded)
            .find_map(|(picture, _)| picture.as_ref().filter(|p| p.item == item));
        let grounded = faded
            .zip(image.as_ref())
            .and_then(|(picture, image)| grounded(picture, image));
        if let Some(grounded) = grounded {
            self.grounded.push((item.clone(), Arc::new(grounded)));
        }
        self.images.push((item, image.map(Arc::new)));
    }

    /// The exercises shown so far whose kind lays its picture under the marks, in order.
    fn laid_under(&self) -> impl Iterator<Item = usize> + '_ {
        (0..self.revealed()).filter(|index| self.facts(*index).is_some_and(|facts| facts.under))
    }

    /// The pictures still to lay under the marks after the `placed` the sheet's picture layer
    /// holds, in order: each as read for the screen (none: it could not be read, or its pack is
    /// gone) and where its frame shows it; up to the first whose picture is not read yet.
    pub(crate) fn under(&self, placed: usize) -> Vec<Under> {
        let mut wanted = Vec::new();
        for index in self.laid_under().skip(placed) {
            let picture = self.sheet.pictures.get(index).and_then(Option::as_ref);
            let image = match picture {
                Some(picture) => {
                    let read = self.images.iter().find(|(item, _)| *item == picture.item);
                    match read {
                        Some((_, image)) => image.clone(),
                        None => break,
                    }
                }
                None => None,
            };
            let frame = self.sheet.pairs[index].copy;
            let corners = match picture {
                Some(picture) => picture.fit.corners().map(|p| frame.to_canvas(p)),
                None => frame.corners(),
            };
            let corners = corners.map(|p| p.map(|v| v as f32));
            let [top_left, _, bottom_right, _] = corners;
            let size = [0, 1].map(|k| bottom_right[k] - top_left[k]);
            wanted.push(Under {
                image,
                place: [top_left, size],
            });
        }
        wanted
    }

    /// Whether a pen landing in exercise `index`'s frame waits for its picture to lie under it,
    /// the sheet's picture layer holding `placed`.
    pub(crate) fn waits_for_picture(&self, index: usize, placed: usize) -> bool {
        let earlier = self.laid_under().take_while(|under| *under <= index);
        let under = self.facts(index).is_some_and(|facts| facts.under);
        under && earlier.count() > placed
    }
}
