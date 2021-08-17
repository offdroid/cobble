use crate::world::BlockType;

#[derive(Copy, Clone)]
pub enum Slot {
    Empty,
    /// Finite number of blocks where the number n is zero-indexed meaning `n = 0` means that there
    /// is one block stored
    Finite(BlockType, u32),
    Infinite(BlockType),
}

const SLOTS: usize = 9;

pub struct Inventory {
    pub slots: [Slot; SLOTS],
    active_slot: usize,
}

impl Inventory {
    pub fn survival_preset() -> Self {
        Self {
            slots: [Slot::Empty; SLOTS],
            active_slot: 0,
        }
    }

    pub fn creative_preset() -> Self {
        Self {
            slots: [
                Slot::Infinite(BlockType::Dirt),
                Slot::Infinite(BlockType::Cobble),
                Slot::Infinite(BlockType::Planks),
                Slot::Infinite(BlockType::Wood),
                Slot::Infinite(BlockType::Bricks),
                Slot::Infinite(BlockType::Gravel),
                Slot::Infinite(BlockType::Sand),
                Slot::Infinite(BlockType::Grass),
                Slot::Infinite(BlockType::Leaves),
            ],
            active_slot: 0,
        }
    }

    pub fn switch_slot(&mut self, slot: usize) {
        assert!(slot < SLOTS);
        self.active_slot = slot;
    }

    pub fn current_slot(&self) -> usize {
        self.active_slot
    }

    pub fn item(&self, slot: usize) -> Option<BlockType> {
        match self.slots[slot] {
            Slot::Finite(b, _) | Slot::Infinite(b) => Some(b),
            _ => None,
        }
    }

    #[allow(dead_code)]
    pub fn current_item(&self) -> Option<BlockType> {
        self.item(self.active_slot)
    }

    pub fn consume_current_slot(&mut self) -> Option<BlockType> {
        self.consume(self.active_slot)
    }

    pub fn consume(&mut self, slot: usize) -> Option<BlockType> {
        match &mut self.slots[slot as usize] {
            Slot::Empty => None,
            Slot::Finite(b, 0) => {
                let b = *b;
                self.slots[slot as usize] = Slot::Empty;
                Some(b)
            }
            Slot::Finite(b, n) => {
                *n -= 1;
                Some(*b)
            }
            Slot::Infinite(b) => Some(*b),
        }
    }

    pub fn absorb(&mut self, block: BlockType, quantity: u32) -> Option<usize> {
        for i in 0..SLOTS {
            match &mut self.slots[i] {
                Slot::Infinite(b) if *b == block => {
                    return Some(i);
                }
                Slot::Finite(b, n) if *b == block => {
                    *n += quantity;
                    return Some(i);
                }
                Slot::Empty => {
                    self.slots[i] = Slot::Finite(block, quantity);
                    return Some(i);
                }
                _ => {}
            }
        }
        None
    }

    pub fn absorb_creative(&mut self, block: BlockType) -> Option<usize> {
        self.slots[self.active_slot] = Slot::Infinite(block);
        Some(self.active_slot)
    }
}
