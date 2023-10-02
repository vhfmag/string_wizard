use crate::{basic_types::AssertIntoU32, chunk::EditOptions, CowStr, MagicString};

#[derive(Debug, Default, Clone)]
pub struct UpdateOptions {
    /// `true` will store the original content in the `name` field of the generated sourcemap.
    pub keep_original: bool,

    /// `true` will clear the `intro` and `outro` for the corresponding range.
    pub overwrite: bool,
}

impl<'text> MagicString<'text> {
    /// A shorthand for `update_with(start, end, content, Default::default())`;
    pub fn update(
        &mut self,
        start: impl AssertIntoU32,
        end: impl AssertIntoU32,
        content: impl Into<CowStr<'text>>,
    ) -> &mut Self {
        self.update_with(start, end, content, Default::default())
    }

    pub fn update_with(
        &mut self,
        start: impl AssertIntoU32,
        end: impl AssertIntoU32,
        content: impl Into<CowStr<'text>>,
        opts: UpdateOptions,
    ) -> &mut Self {
        self.update_with_inner(
            start.assert_into_u32(),
            end.assert_into_u32(),
            content.into(),
            opts,
            true,
        );
        self
    }

    pub fn remove(&mut self, start: impl AssertIntoU32, end: impl AssertIntoU32) -> &mut Self {
        self.update_with_inner(
            start.assert_into_u32(),
            end.assert_into_u32(),
            "".into(),
            UpdateOptions {
                keep_original: false,
                overwrite: true,
            },
            false,
        );

        self
    }

    // --- private

    fn update_with_inner(
        &mut self,
        start: u32,
        end: u32,
        content: CowStr<'text>,
        opts: UpdateOptions,
        panic_if_start_equal_end: bool,
    ) -> &mut Self {
        let start = start as u32;
        let end = end as u32;
        if panic_if_start_equal_end && start == end {
            panic!(
                "Cannot overwrite a zero-length range – use append_left or prepend_right instead"
            )
        }
        assert!(start < end);
        self.split_at(start);
        self.split_at(end);

        let start_idx = self.chunk_by_start.get(&start).copied().unwrap();
        let end_idx = self.chunk_by_end.get(&end).copied().unwrap();

        let start_chunk = &mut self.chunks[start_idx];
        start_chunk.edit(
            content.into(),
            EditOptions {
                overwrite: opts.overwrite,
                store_name: opts.keep_original,
            },
        );

        let mut rest_chunk_idx = if start_idx != end_idx {
            start_chunk.next.unwrap()
        } else {
            return self;
        };

        while rest_chunk_idx != end_idx {
            let rest_chunk = &mut self.chunks[rest_chunk_idx];
            rest_chunk.edit("".into(), Default::default());
            rest_chunk_idx = rest_chunk.next.unwrap();
        }
        self
    }
}
