use crate::model::YesNo;

impl From<bool> for YesNo {
    fn from(value: bool) -> Self {
        match value {
            true => YesNo::Yes,
            false => YesNo::No,
        }
    }
}
impl From<YesNo> for bool {
    fn from(value: YesNo) -> Self {
        match value {
            YesNo::Yes => true,
            YesNo::No => false,
        }
    }
}
