/// Merkle selection.
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum MerkleSelection {
    /// Choose left at current depth.
    Left,
    /// Choose right at current depth.
    Right,
}

/// Merkle route.
#[derive(Clone, Eq, PartialEq, Debug)]
pub enum MerkleRoute {
    /// Root of the merkle tree.
    Root,
    /// Select items from the root.
    Select(Vec<MerkleSelection>),
}

impl MerkleRoute {
    /// Get selection at depth, where root is considered depth 0.
    pub fn at_depth(&self, depth: usize) -> Option<MerkleSelection> {
        match self {
            MerkleRoute::Root => None,
            MerkleRoute::Select(selections) => {
                if depth == 0 || depth > selections.len() {
                    None
                } else {
                    Some(selections[depth - 1])
                }
            },
        }
    }
}

/// Raw merkle index.
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct MerkleIndex(usize);

impl MerkleIndex {
    /// Root merkle index.
    pub const fn root() -> Self {
        Self(1)
    }

    /// Get left child of current index.
    pub const fn left(&self) -> Self {
        Self(2 * self.0)
    }

    /// Get right child of current index.
    pub const fn right(&self) -> Self {
        Self(2 * self.0 + 1)
    }

    /// Get the parent of current merkle index.
    pub fn parent(&self) -> Option<Self> {
        if self.0 == 1 {
            None
        } else {
            Some(Self(self.0 / 2))
        }
    }

    /// From one-based index.
    pub fn from_one(value: usize) -> Option<Self> {
        if value == 0 {
            None
        } else {
            Some(Self(value))
        }
    }

    /// From zero-based index.
    pub fn from_zero(value: usize) -> Self {
        Self(value + 1)
    }

    /// Get selections from current index.
    pub fn route(&self) -> MerkleRoute {
        let mut value = self.0;
        let mut selections = Vec::<MerkleSelection>::new();

        loop {
            if value >> 1 == 0 {
                debug_assert!(value == 1);

                if selections.is_empty() {
                    return MerkleRoute::Root
                } else {
                    selections.reverse();
                    return MerkleRoute::Select(selections)
                }
            }

            let sel = value & 0b1;
            if sel == 0 {
                selections.push(MerkleSelection::Left)
            } else {
                selections.push(MerkleSelection::Right)
            }

            value = value >> 1;
        }
    }
}
