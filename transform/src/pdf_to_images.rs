use crate::{Color, PageRange, Quality, Result, TransformationState};
use serde::Serialize;
use serde_json;
use std::{convert::TryInto, io, mem};

const HEADER_PREFIX: &'static [u8] = b"PPDF";
const HEADER_OFFSET_BYTES: usize = mem::size_of::<u32>();
const HEADER_POSTFIX_SIZE: usize = 3;
const HEADER_IMG_POSTFIX: &'static [u8; HEADER_POSTFIX_SIZE] = b"IMG";
const HEADER_META_POSTFIX: &'static [u8; HEADER_POSTFIX_SIZE] = b"MET";
const HEADER_SIZE: usize = HEADER_PREFIX.len() + HEADER_OFFSET_BYTES + HEADER_POSTFIX_SIZE;

fn transform(
    in_blob: Vec<u8>,
    selected_page_range: Option<PageRange>,
    quality: Quality,
    background_color: Option<Color>,
) -> Result<Images> {
    TransformationState::try_new(in_blob, selected_page_range, quality, background_color)
        .map(|transformation| Images::new(transformation))
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ImageHeader {
    size: u32,
    postfix: &'static [u8; HEADER_POSTFIX_SIZE],
}

impl ImageHeader {
    fn try_new(
        postfix: &'static [u8; HEADER_POSTFIX_SIZE],
        size: usize,
    ) -> io::Result<ImageHeader> {
        let size = size
            .try_into()
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;

        Ok(ImageHeader { size, postfix })
    }

    fn to_bytes(&self) -> io::Result<Vec<u8>> {
        let header_size_u32: u32 = HEADER_SIZE
            .try_into()
            .map_err(|error| io::Error::new(io::ErrorKind::Other, error))?;
        let offset_to_next_header: u32 = self.size + header_size_u32;
        let offset_to_next_header: [u8; 4] = offset_to_next_header.to_be_bytes();
        let mut header = Vec::with_capacity(HEADER_SIZE);
        header.extend(HEADER_PREFIX);
        header.extend_from_slice(&offset_to_next_header);
        header.extend(self.postfix);
        Ok(header)
    }
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct ImagesMetadata {
    original_title: String,
    page_count: usize,
}

#[derive(Debug)]
struct Images {
    transformation: TransformationState,
    /// Bytes that have yet to be read, in reverse order such that one could get
    // the first three bytes in order with
    /// [unread.pop(), unread.pop(), unread.pop()]
    unread: Vec<u8>,
    /// Offset from the range start to the next page to transform
    next_page: usize,
    has_queued_metadata: bool,
}

impl Images {
    fn new(transformation: TransformationState) -> Images {
        Images {
            transformation,
            unread: Vec::new(),
            next_page: 0,
            has_queued_metadata: false,
        }
    }
}

impl io::Read for Images {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let Images {
            transformation: trans,
            unread,
            next_page,
            has_queued_metadata,
        } = self;
        let range = trans.options.page_range;

        if !*has_queued_metadata {
            let meta = ImagesMetadata {
                original_title: trans.doc.original_title.clone(),
                page_count: trans.doc.page_count,
            };
            let mut meta = serde_json::to_vec(&meta)
                .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))?;
            meta.reverse();

            let mut header = ImageHeader::try_new(HEADER_META_POSTFIX, meta.len())?.to_bytes()?;
            header.reverse();

            unread.extend(meta);
            unread.extend(header);

            *has_queued_metadata = true;
        }

        if unread.len() == 0 && !range.includes(*next_page, trans.doc.page_count) {
            // finished transforming, so nothing we can output
            return Ok(0);
        }

        if unread.len() == 0 {
            // transform another page
            let mut image = trans
                .transform_page(*next_page)
                .and_then(|page| page.to_png())
                .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
            image.reverse();

            let mut header = ImageHeader::try_new(HEADER_IMG_POSTFIX, image.len())?.to_bytes()?;
            header.reverse();

            unread.extend(image);
            unread.extend(header);
            *next_page += 1;
        }

        let mut bytes_read = 0;

        for slot in buf.iter_mut() {
            let next = unread.pop();
            if next.is_none() {
                break;
            }
            *slot = next.unwrap();
            bytes_read += 1;
        }

        Ok(bytes_read)
    }
}

pub fn transform_page(
    in_blob: Vec<u8>,
    page: usize,
    quality: Quality,
    background_color: Option<Color>,
) -> Result<Vec<u8>> {
    TransformationState::try_new(in_blob, None, quality, background_color)
        .and_then(|trans| trans.transform_page(page))
        .and_then(|page| page.to_png())
}

#[cfg(test)]
mod test {
    use super::*;
    use std::io::Read;

    fn get_in_blob() -> Vec<u8> {
        include_bytes!("../test_assets/multipage_test.pdf").to_vec()
    }

    fn get_unchecked() -> Images {
        transform(get_in_blob(), None, Quality::High, None).unwrap()
    }

    #[test]
    fn can_create() {
        get_unchecked();
    }

    #[test]
    fn can_read() {
        let mut images = get_unchecked();
        const BUFFER_SIZE: usize = 1000;
        loop {
            let mut buf = [0; BUFFER_SIZE];
            let bytes_read = images.read(&mut buf).unwrap();
            if bytes_read == 0 {
                break;
            }
        }
    }

    #[test]
    fn valid_headers() {
        let mut images = get_unchecked();
        let mut buf = Vec::new();
        images.read_to_end(&mut buf).unwrap();

        let mut i = 0;
        let mut meta_count = 0;
        loop {
            if i == buf.len() {
                break;
            } else if i > buf.len() {
                panic!("Attempting to read past end");
            }

            let start = i;
            let stop = start + HEADER_PREFIX.len();
            assert!(buf[start..stop].iter().eq(HEADER_PREFIX));

            let start = stop;
            let stop = start + HEADER_OFFSET_BYTES;
            let offset: [u8; 8] = buf[start..stop].try_into().unwrap();
            let offset = u64::from_be_bytes(offset);
            i += offset as usize;

            let start = stop;
            let stop = start + HEADER_POSTFIX_SIZE;
            let range = &buf[start..stop];
            assert!(range.iter().eq(HEADER_META_POSTFIX) || range.iter().eq(HEADER_IMG_POSTFIX));
            if range.iter().eq(HEADER_META_POSTFIX) {
                meta_count += 1;
            }
        }
        assert_eq!(meta_count, 1);
    }
}
