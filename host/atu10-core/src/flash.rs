use crate::hex::HexImage;
use crate::{Error, Result};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlashRow {
    pub base_address: u32,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlashPlan {
    pub row_size: usize,
    pub fill: u8,
    pub rows: Vec<FlashRow>,
}

impl FlashPlan {
    pub fn from_image(image: &HexImage, row_size: usize, fill: u8) -> Result<Self> {
        if row_size == 0 || !row_size.is_power_of_two() {
            return Err(Error::Device(
                "row size must be a non-zero power of two".to_string(),
            ));
        }

        let mut rows: BTreeMap<u32, Vec<u8>> = BTreeMap::new();
        for (address, value) in image.bytes() {
            let base = address - (address % row_size as u32);
            let row = rows.entry(base).or_insert_with(|| vec![fill; row_size]);
            row[(address - base) as usize] = *value;
        }

        Ok(Self {
            row_size,
            fill,
            rows: rows
                .into_iter()
                .map(|(base_address, data)| FlashRow { base_address, data })
                .collect(),
        })
    }

    pub fn verify_image(&self) -> BTreeMap<u32, u8> {
        let mut bytes = BTreeMap::new();
        for row in &self.rows {
            for (index, value) in row.data.iter().enumerate() {
                bytes.insert(row.base_address + index as u32, *value);
            }
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
            ":08000200112233445566778892\n\
             :02001400AABB85\n\
             :00000001FF\n",
        )
        .unwrap();

        let plan = FlashPlan::from_image(&image, 8, 0xff).unwrap();

        assert_eq!(plan.rows.len(), 3);
        assert_eq!(plan.rows[0].base_address, 0x00);
        assert_eq!(
            plan.rows[0].data,
            vec![0xff, 0xff, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66]
        );
        assert_eq!(plan.rows[1].base_address, 0x08);
        assert_eq!(
            plan.rows[1].data,
            vec![0x77, 0x88, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]
        );
        assert_eq!(plan.rows[2].base_address, 0x10);
        assert_eq!(
            plan.rows[2].data,
            vec![0xff, 0xff, 0xff, 0xff, 0xaa, 0xbb, 0xff, 0xff]
        );
    }

    #[test]
    fn rejects_invalid_row_size() {
        let image = HexImage::parse(":00000001FF\n").unwrap();
        assert!(FlashPlan::from_image(&image, 0, 0xff).is_err());
        assert!(FlashPlan::from_image(&image, 7, 0xff).is_err());
    }
}
