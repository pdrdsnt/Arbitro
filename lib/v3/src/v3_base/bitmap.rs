use alloy_primitives::{B256, U256, aliases::I24};

use crate::v3_base::bitmap_math;

#[derive(Debug, Clone)]
pub struct BitMap {
    bitmap: Vec<Option<U256>>,
}

impl BitMap {
    pub fn new(tick_spacing: I24, words: Vec<(i16, U256)>) -> Self {
        println!("tick spacing {}:", tick_spacing);
        let total_words = (65536_i64 + tick_spacing.as_i64())
            .checked_div(tick_spacing.as_i64())
            .expect("problem dividing full bitmap range by tick spacing ");

        let mut bm = Vec::with_capacity(total_words as usize);

        for _ in 0..total_words {
            bm.push(None);
        }
        for (pos, word) in words {
            let idx = (pos + i16::MAX) as usize;
            bm[idx] = Some(word);
        }

        BitMap { bitmap: bm }
    }
    pub fn get_word_from_pos(&self, word_pos: i16, tick_spacing: I24) -> Option<U256> {
        let index = Self::pos_to_idx(word_pos, tick_spacing);
        self.bitmap[index]
    }

    pub fn get_pos_from_tick(&self, tick: I24, tick_spacing: I24) -> i16 {
        let normalized_tick = bitmap_math::normalize_tick(tick, tick_spacing);
        bitmap_math::word_index(normalized_tick)
    }

    pub fn get_word_from_tick(&self, tick: I24, tick_spacing: I24) -> Option<U256> {
        let normalized_tick = bitmap_math::normalize_tick(tick, tick_spacing);
        let word_pos = bitmap_math::word_index(normalized_tick);
        self.get_word_from_pos(word_pos, tick_spacing)
    }

    pub fn insert(&mut self, pos: i16, word: U256, tick_spacing: I24) {
        self.bitmap[Self::pos_to_idx(pos, tick_spacing)] = Some(word);
    }
    pub fn pos_to_idx(word_pos: i16, tick_spacing: I24) -> usize {
        println!("word pos: {} , tick spacing: {}", word_pos, tick_spacing);
        (word_pos as isize + (i16::max_value() as isize) as isize / tick_spacing.as_isize())
            as usize
    }
}
