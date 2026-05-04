use crate::hex::HexImage;
use crate::{Error, Result};
use std::collections::BTreeMap;

pub const F18877_PFM_WORDS: u32 = 0x8000;
pub const F18877_ROW_WORDS: usize = 32;
pub const F18877_ERASED_WORD: u16 = 0x3fff;
const F18877_PFM_BYTES: u32 = F18877_PFM_WORDS * 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlashRow {
    pub base_word_address: u32,
    pub words: Vec<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlashPlan {
    pub row_words: usize,
    pub fill_word: u16,
    pub rows: Vec<FlashRow>,
}

impl FlashPlan {
    pub fn from_image(image: &HexImage, row_words: usize, fill_word: u16) -> Result<Self> {
        if row_words == 0 || !row_words.is_power_of_two() {
            return Err(Error::Device(
                "row word count must be a non-zero power of two".to_string(),
            ));
        }

        let mut rows: BTreeMap<u32, Vec<u16>> = BTreeMap::new();
        for (byte_address, value) in image.bytes() {
            if *byte_address >= F18877_PFM_BYTES {
                return Err(Error::Device(format!(
                    "HEX address 0x{byte_address:08x} is outside PIC16F18877 program flash; config/user/eeprom writes are intentionally rejected"
                )));
            }

            let word_address = byte_address / 2;
            let base = word_address - (word_address % row_words as u32);
            let row = rows
                .entry(base)
                .or_insert_with(|| vec![fill_word; row_words]);
            let index = (word_address - base) as usize;
            if byte_address % 2 == 0 {
                row[index] = (row[index] & 0x3f00u16) | u16::from(*value);
            } else {
                row[index] = (row[index] & 0x00ffu16) | ((u16::from(*value) & 0x3fu16) << 8);
            }
        }

        Ok(Self {
            row_words,
            fill_word,
            rows: rows
                .into_iter()
                .map(|(base_word_address, words)| FlashRow {
                    base_word_address,
                    words,
                })
                .collect(),
        })
    }

    pub fn f18877_from_image(image: &HexImage) -> Result<Self> {
        Self::from_image(image, F18877_ROW_WORDS, F18877_ERASED_WORD)
    }

    pub fn verify_words(&self) -> BTreeMap<u32, u16> {
        let mut words = BTreeMap::new();
        for row in &self.rows {
            for (index, value) in row.words.iter().enumerate() {
                words.insert(
                    row.base_word_address + index as u32,
                    *value & F18877_ERASED_WORD,
                );
            }
        }
        words
    }
}

impl FlashRow {
    pub fn packed_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.words.len() * 2);
        for word in &self.words {
            let masked = word & F18877_ERASED_WORD;
            bytes.push((masked & 0x00ff) as u8);
            bytes.push((masked >> 8) as u8);
        }
        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hex::HexImage;

    #[test]
    fn rows_are_aligned_and_gap_filled() {
        let image = HexImage::parse(
            ":04000400112233444E\n\
             :02004000AABB59\n\
             :00000001FF\n",
        )
        .unwrap();

        let plan = FlashPlan::from_image(&image, 8, F18877_ERASED_WORD).unwrap();

        assert_eq!(plan.rows.len(), 2);
        assert_eq!(plan.rows[0].base_word_address, 0x00);
        assert_eq!(
            plan.rows[0].words,
            vec![0x3fff, 0x3fff, 0x2211, 0x0433, 0x3fff, 0x3fff, 0x3fff, 0x3fff]
        );
        assert_eq!(plan.rows[1].base_word_address, 0x20);
        assert_eq!(
            plan.rows[1].words,
            vec![0x3baa, 0x3fff, 0x3fff, 0x3fff, 0x3fff, 0x3fff, 0x3fff, 0x3fff]
        );
    }

    #[test]
    fn rejects_invalid_row_size() {
        let image = HexImage::parse(":00000001FF\n").unwrap();
        assert!(FlashPlan::from_image(&image, 0, F18877_ERASED_WORD).is_err());
        assert!(FlashPlan::from_image(&image, 7, F18877_ERASED_WORD).is_err());
    }

    #[test]
    fn rejects_config_or_user_id_regions_by_default() {
        let image = HexImage::parse(
            ":020000040001F9\n\
             :02000E00FF3FB2\n\
             :00000001FF\n",
        )
        .unwrap();

        let err = FlashPlan::f18877_from_image(&image).unwrap_err();
        assert!(err.to_string().contains("config"));
    }

    #[test]
    fn packs_words_little_endian_for_icsp_transfer() {
        let row = FlashRow {
            base_word_address: 0,
            words: vec![0x1234, 0x3fff],
        };

        assert_eq!(row.packed_bytes(), vec![0x34, 0x12, 0xff, 0x3f]);
    }
}
