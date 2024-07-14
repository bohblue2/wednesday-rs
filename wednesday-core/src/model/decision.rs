#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Long,
    CloseLong,
    Short,
    CloseShort,
    Hold,
}

impl Default for Decision {
    fn default() -> Self {
        Decision::Hold
    }
}

impl Decision {
    pub fn is_long(&self) -> bool {
        matches!(self, Decision::Long)
    }

    pub fn is_short(&self) -> bool {
        matches!(self, Decision::Short)
    }

    pub fn is_entry(&self) -> bool {
        matches!(self, Decision::Long | Decision::Short)
    }

    pub fn is_exit(&self) -> bool {
        matches!(self, Decision::CloseLong | Decision::CloseShort)
    }
}
