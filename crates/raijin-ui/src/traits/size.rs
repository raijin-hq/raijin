use inazuma::{DefiniteLength, Edges, Pixels, Styled, px};
use serde::{Deserialize, Serialize};

/// A size for elements.
#[derive(Clone, Default, Copy, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub enum Size {
    Size(Pixels),
    XSmall,
    Small,
    #[default]
    Medium,
    Large,
}

impl Size {
    fn as_f32(&self) -> f32 {
        match self {
            Size::Size(val) => val.as_f32(),
            Size::XSmall => 0.,
            Size::Small => 1.,
            Size::Medium => 2.,
            Size::Large => 3.,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Size::XSmall => "xs",
            Size::Small => "sm",
            Size::Medium => "md",
            Size::Large => "lg",
            Size::Size(_) => "custom",
        }
    }

    pub fn from_str(size: &str) -> Self {
        match size.to_lowercase().as_str() {
            "xs" | "xsmall" => Size::XSmall,
            "sm" | "small" => Size::Small,
            "md" | "medium" => Size::Medium,
            "lg" | "large" => Size::Large,
            _ => Size::Medium,
        }
    }

    pub fn table_row_height(&self) -> Pixels {
        match self {
            Size::XSmall => px(26.),
            Size::Small => px(30.),
            Size::Large => px(40.),
            _ => px(32.),
        }
    }

    pub fn table_cell_padding(&self) -> Edges<Pixels> {
        match self {
            Size::XSmall => Edges { top: px(2.), bottom: px(2.), left: px(4.), right: px(4.) },
            Size::Small => Edges { top: px(3.), bottom: px(3.), left: px(6.), right: px(6.) },
            Size::Large => Edges { top: px(8.), bottom: px(8.), left: px(12.), right: px(12.) },
            _ => Edges { top: px(4.), bottom: px(4.), left: px(8.), right: px(8.) },
        }
    }

    pub fn smaller(&self) -> Self {
        match self {
            Size::XSmall => Size::XSmall,
            Size::Small => Size::XSmall,
            Size::Medium => Size::Small,
            Size::Large => Size::Medium,
            Size::Size(val) => Size::Size(*val * 0.2),
        }
    }

    pub fn larger(&self) -> Self {
        match self {
            Size::XSmall => Size::Small,
            Size::Small => Size::Medium,
            Size::Medium => Size::Large,
            Size::Large => Size::Large,
            Size::Size(val) => Size::Size(*val * 1.2),
        }
    }

    pub fn max(&self, other: Self) -> Self {
        match (self, other) {
            (Size::Size(a), Size::Size(b)) => Size::Size(px(a.as_f32().min(b.as_f32()))),
            (Size::Size(a), _) => Size::Size(*a),
            (_, Size::Size(b)) => Size::Size(b),
            (a, b) if a.as_f32() < b.as_f32() => *a,
            _ => other,
        }
    }

    pub fn min(&self, other: Self) -> Self {
        match (self, other) {
            (Size::Size(a), Size::Size(b)) => Size::Size(px(a.as_f32().max(b.as_f32()))),
            (Size::Size(a), _) => Size::Size(*a),
            (_, Size::Size(b)) => Size::Size(b),
            (a, b) if a.as_f32() > b.as_f32() => *a,
            _ => other,
        }
    }

    pub fn input_px(&self) -> Pixels {
        match self {
            Self::Large => px(16.),
            Self::Medium => px(12.),
            Self::Small => px(8.),
            Self::XSmall => px(4.),
            _ => px(8.),
        }
    }

    pub fn input_py(&self) -> Pixels {
        match self {
            Size::Large => px(10.),
            Size::Medium => px(8.),
            Size::Small => px(2.),
            Size::XSmall => px(0.),
            _ => px(2.),
        }
    }
}

impl From<Pixels> for Size {
    fn from(size: Pixels) -> Self {
        Size::Size(size)
    }
}

/// A trait for setting the size of an element.
pub trait Sizable: Sized {
    fn with_size(self, size: impl Into<Size>) -> Self;

    fn xsmall(self) -> Self {
        self.with_size(Size::XSmall)
    }

    fn small(self) -> Self {
        self.with_size(Size::Small)
    }

    fn large(self) -> Self {
        self.with_size(Size::Large)
    }
}

/// Convenience sizing methods for Styled elements.
pub trait StyleSized: Styled + Sized {
    fn input_text_size(self, size: Size) -> Self {
        match size {
            Size::XSmall => self.text_xs(),
            Size::Small | Size::Medium => self.text_sm(),
            Size::Large => self.text_base(),
            Size::Size(size) => self.text_size(size * 0.875),
        }
    }

    fn input_size(self, size: Size) -> Self {
        self.input_px(size).input_py(size).input_h(size)
    }

    fn input_pl(self, size: Size) -> Self {
        self.pl(size.input_px())
    }

    fn input_pr(self, size: Size) -> Self {
        self.pr(size.input_px())
    }

    fn input_px(self, size: Size) -> Self {
        self.px(size.input_px())
    }

    fn input_py(self, size: Size) -> Self {
        self.py(size.input_py())
    }

    fn input_h(self, size: Size) -> Self {
        match size {
            Size::Large => self.h_11(),
            Size::Medium => self.h_8(),
            Size::Small => self.h_6(),
            Size::XSmall => self.h_5(),
            _ => self.h_6(),
        }
    }

    fn list_size(self, size: Size) -> Self {
        self.list_px(size).list_py(size).input_text_size(size)
    }

    fn list_px(self, size: Size) -> Self {
        match size {
            Size::Small => self.px_2(),
            _ => self.px_3(),
        }
    }

    fn list_py(self, size: Size) -> Self {
        match size {
            Size::Large => self.py_2(),
            Size::Medium => self.py_1(),
            Size::Small => self.py_0p5(),
            _ => self.py_1(),
        }
    }

    fn size_with(self, size: Size) -> Self {
        match size {
            Size::Large => self.size_11(),
            Size::Medium => self.size_8(),
            Size::Small => self.size_5(),
            Size::XSmall => self.size_4(),
            Size::Size(size) => self.size(size),
        }
    }

    fn table_cell_size(self, size: Size) -> Self {
        let padding = size.table_cell_padding();
        match size {
            Size::XSmall | Size::Small => self.text_sm(),
            _ => self,
        }
        .pl(padding.left)
        .pr(padding.right)
        .pt(padding.top)
        .pb(padding.bottom)
    }

    fn button_text_size(self, size: Size) -> Self {
        match size {
            Size::XSmall => self.text_xs(),
            Size::Small => self.text_sm(),
            _ => self.text_base(),
        }
    }
}

impl<T: Styled> StyleSized for T {}

#[cfg(test)]
mod tests {
    use inazuma::px;
    use super::Size;

    #[test]
    fn test_size_max_min() {
        assert_eq!(Size::Small.min(Size::XSmall), Size::Small);
        assert_eq!(Size::XSmall.min(Size::Small), Size::Small);
        assert_eq!(Size::Large.min(Size::Small), Size::Large);
        assert_eq!(Size::Size(px(10.)).min(Size::Size(px(20.))), Size::Size(px(20.)));

        assert_eq!(Size::Small.max(Size::XSmall), Size::XSmall);
        assert_eq!(Size::XSmall.max(Size::Small), Size::XSmall);
        assert_eq!(Size::Size(px(10.)).max(Size::Size(px(20.))), Size::Size(px(10.)));
    }

    #[test]
    fn test_size_as_str() {
        assert_eq!(Size::XSmall.as_str(), "xs");
        assert_eq!(Size::Small.as_str(), "sm");
        assert_eq!(Size::Medium.as_str(), "md");
        assert_eq!(Size::Large.as_str(), "lg");
    }

    #[test]
    fn test_size_from_str() {
        assert_eq!(Size::from_str("xs"), Size::XSmall);
        assert_eq!(Size::from_str("SMALL"), Size::Small);
        assert_eq!(Size::from_str("unknown"), Size::Medium);
    }
}
