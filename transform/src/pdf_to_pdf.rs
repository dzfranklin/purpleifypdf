use crate::{Color, PageRange, Quality, Result, TransformationState, TransformedPage};

pub fn transform(
    in_blob: Vec<u8>,
    selected_page_range: Option<PageRange>,
    quality: Quality,
    background_color: Option<Color>,
) -> Result<Progress> {
    TransformationState::try_new(in_blob, selected_page_range, quality, background_color)
        .map(|state| Progress::new(0, Vec::new(), state))
}

pub enum Update {
    Progress(Progress),
    Complete(Result<Complete>),
}

pub struct Complete {
    original_title: String,
    bytes: Vec<u8>,
}

impl Complete {
    fn new(original_title: String, bytes: Vec<u8>) -> Self {
        Complete {
            original_title,
            bytes,
        }
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    pub fn original_title(&self) -> &str {
        self.original_title.as_str()
    }
}

pub struct Progress {
    percent: f64,
    state: TransformationState,
    transformed_pages: Vec<TransformedPage>,
    /// The offset from the start of the range to the next page to transform
    next_offset: usize,
}

impl Progress {
    fn new(
        next_offset: usize,
        transformed_pages: Vec<TransformedPage>,
        state: TransformationState,
    ) -> Self {
        // We add one to the rhs to account for the fact that we aren't done
        // after we process the last page, there's one more step.
        let percent = next_offset as f64 / (state.options.page_range.count + 1) as f64;

        Progress {
            next_offset,
            percent,
            transformed_pages,
            state,
        }
    }

    pub fn percent_done(&self) -> f64 {
        self.percent
    }

    pub fn next(self) -> Update {
        let Progress {
            next_offset,
            mut transformed_pages,
            state,
            ..
        } = self;
        let range = &state.options.page_range;

        if range.includes(next_offset, state.doc.page_count) {
            let page = state.transform_page(next_offset);
            page.map(|page| {
                transformed_pages.push(page);
                Update::Progress(Progress::new(next_offset + 1, transformed_pages, state))
            })
            .unwrap_or_else(|err| Update::Complete(Err(err)))
        } else {
            let original_title = state.doc.original_title.clone();
            Update::Complete(
                state
                    .to_pdf(transformed_pages)
                    .map(|bytes| Complete::new(original_title, bytes)),
            )
        }
    }

    pub fn finish(self) -> Result<Complete> {
        let mut state = self;
        loop {
            match state.next() {
                Update::Progress(next) => state = next,
                Update::Complete(result) => return result,
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    // TODO: Add visual tests. Right now all we test is that some output bytes are produced.
    fn get_in_blob() -> Vec<u8> {
        include_bytes!("../test_assets/multipage_test.pdf").to_vec()
    }

    fn transform_unchecked_finish(page_range: Option<PageRange>, quality: Quality) -> Vec<u8> {
        transform(get_in_blob(), page_range, quality, None)
            .unwrap()
            .finish()
            .unwrap()
            .into_bytes()
    }

    #[test]
    fn transforms_pdf() {
        let out_blob = transform_unchecked_finish(
            Some(PageRange {
                starting_index: 0,
                count: 1,
            }),
            Quality::ExtremeLow,
        );
        assert!(!out_blob.is_empty());
    }

    #[test]
    fn transforms_pdf_high_quality() {
        let out_blob = transform_unchecked_finish(
            Some(PageRange {
                starting_index: 0,
                count: 1,
            }),
            Quality::High,
        );
        assert!(!out_blob.is_empty());
    }

    #[test]
    fn handles_page_range() {
        let out_blob_entire = transform_unchecked_finish(None, Quality::ExtremeLow);

        let out_blob_partial = transform_unchecked_finish(
            Some(PageRange {
                starting_index: 0,
                count: 1,
            }),
            Quality::ExtremeLow,
        );

        let out_blob_none = transform_unchecked_finish(
            Some(PageRange {
                starting_index: 0,
                count: 0,
            }),
            Quality::ExtremeLow,
        );

        let out_blob_none_2 = transform_unchecked_finish(
            Some(PageRange {
                starting_index: 100,
                count: 10,
            }),
            Quality::ExtremeLow,
        );

        let out_blob_part_of_range = transform_unchecked_finish(
            Some(PageRange {
                starting_index: 0,
                count: 100,
            }),
            Quality::ExtremeLow,
        );

        assert_eq!(out_blob_none.len(), out_blob_none_2.len());
        assert!(out_blob_entire.len() > out_blob_partial.len());
        assert!(out_blob_partial.len() > out_blob_none.len());
        assert_eq!(out_blob_part_of_range.len(), out_blob_entire.len())
    }

    #[test]
    fn handles_quality() {
        let qualities = [
            Quality::ExtremeLow,
            Quality::Low,
            // Quality::Normal,
            // Quality::High,
            // Quality::Extreme,
        ];
        let lengths = qualities
            .iter()
            .map(|quality| {
                transform_unchecked_finish(
                    Some(PageRange {
                        starting_index: 0,
                        count: 1,
                    }),
                    *quality,
                )
                .len()
            })
            .collect::<Vec<usize>>();

        assert!(lengths.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn provides_updates() {
        use poppler::PopplerDocument;

        let mut prev_percent = 0.0;
        let mut iterations = 0;
        let max_iterations = PopplerDocument::new_from_data(&mut get_in_blob(), "")
            .unwrap()
            .get_n_pages();

        let mut state = transform(get_in_blob(), None, Quality::ExtremeLow, None).unwrap();
        loop {
            if iterations > max_iterations {
                panic!("Too many updates for document size");
            }

            match state.next() {
                Update::Progress(progress) => {
                    let percent = progress.percent_done();
                    if prev_percent >= percent {
                        panic!("Progress must always increase in TransformationUpdate::Progress(progress)")
                    } else {
                        prev_percent = percent;
                    }
                    state = progress;
                }
                Update::Complete(result) => {
                    result.unwrap();
                    break;
                }
            }

            iterations += 1;
        }

        if iterations < max_iterations {
            panic!("Too few updates for document size");
        }
    }
}
