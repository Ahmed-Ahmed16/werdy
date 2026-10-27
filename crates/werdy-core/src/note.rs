use crate::{Error, Fraction, WerdyDay, mark};

/// Credit applied to each due deed on a werdy day.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExcuseCredit {
    /// No credit. Marks keep their own values.
    None,
    /// Each due deed is worth at least one half.
    Half,
    /// Each due deed is worth one.
    Full,
}

impl ExcuseCredit {
    /// Returns the fraction this credit gives each due deed.
    pub fn value(self) -> f64 {
        match self {
            Self::None => 0.0,
            Self::Half => 0.5,
            Self::Full => 1.0,
        }
    }

    pub(crate) fn exact_value(self) -> Fraction {
        match self {
            Self::None => Fraction::ZERO,
            Self::Half => Fraction::HALF,
            Self::Full => Fraction::ONE,
        }
    }
}

/// Owner's note and optional credit for one werdy day.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DayNote {
    text: String,
    credit: ExcuseCredit,
    entered_on: WerdyDay,
}

impl DayNote {
    /// Creates a note. Empty text needs half or full credit.
    pub fn new(text: &str, credit: ExcuseCredit, entered_on: WerdyDay) -> Result<Self, Error> {
        if text.is_empty() && credit == ExcuseCredit::None {
            return Err(Error::EmptyDayNote);
        }
        Ok(Self {
            text: text.to_owned(),
            credit,
            entered_on,
        })
    }

    /// Returns the owner's text.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Returns the credit chosen for this day.
    pub fn credit(&self) -> ExcuseCredit {
        self.credit
    }

    /// Returns the werdy day when the note was entered.
    pub fn entered_on(&self) -> WerdyDay {
        self.entered_on
    }

    /// Says whether the note was entered more than two days after its day.
    pub fn is_late_for(&self, day: WerdyDay) -> bool {
        mark::is_late_for(self.entered_on, day)
    }
}
