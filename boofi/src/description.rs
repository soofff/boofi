pub(crate) use boofi_macros::Description;
use serde::Serialize;

/// Description about in and output with their types, fields and name
/// Use derive(Description) if possible
pub(crate) trait Description {
    const KIND: &'static str = "unknown";
    const NAME: &'static str = Self::KIND;
    const DESCRIPTION: &'static str = "";
    const FIELDS: &'static [DescriptionField] = &[];

    fn field() -> &'static DescriptionField {
        &DescriptionField {
            kind: Self::KIND,
            name: Self::NAME,
            description: Self::DESCRIPTION,
            fields: Self::FIELDS,
        }
    }
}

/// The actual field description
#[derive(Debug, Serialize)]
pub(crate) struct DescriptionField {
    pub(crate) kind: &'static str,
    pub(crate) name: &'static str,
    pub(crate) description: &'static str,
    pub(crate) fields: &'static [Self],
}

macro_rules! description {
    (
        $typ:ty
    ) => {
        impl crate::description::Description for $typ {
            const KIND: &'static str = stringify!($typ);
        }
    };
    (
        $typ:ty,
        $kind:literal
    ) => {
        impl crate::description::Description for $typ {
            const KIND: &'static str = $kind;
        }
    };
    (
        $typ:ty,
        $kind:literal,
        $name:literal
    ) => {
        impl crate::description::Description for $typ {
            const KIND: &'static str = $kind;
            const NAME: &'static str = $name;
        }
    }
}

macro_rules! description_field_generic {
    () => {
        const FIELDS: &'static [DescriptionField] = &[DescriptionField {
            kind: T::KIND,
            name: T::NAME,
            description: T::DESCRIPTION,
            fields: T::FIELDS,
        }];
    }
}

description!(bool);
description!(usize);
description!(isize);
description!(f32);
description!(f64);
description!(String);
description!((), "empty");
description!((bool, String));

impl<T: Description> Description for Option<T> {
    const KIND: &'static str = "optional";
    const NAME: &'static str = "optional (see fields)";
    const DESCRIPTION: &'static str = "use eventually fields";
    description_field_generic!();
}

impl<T: Description> Description for Vec<T> {
    const KIND: &'static str = "array";
    description_field_generic!();
}

#[cfg(test)]
mod test {
    use boofi_macros::Description;
    use crate::description::*;

    #[allow(dead_code)]
    #[derive(Description)]
    enum Third {
        A(bool)
    }

    #[allow(dead_code)]
    #[derive(Description)]
    struct Second<T> {
        aa: T,
        #[desc(kind = "text", name = "b²", description = "double b")]
        bb: String,
    }

    #[allow(dead_code)]
    #[derive(Description)]
    #[desc(kind = "1st", name = "1", description = "start")]
    struct First {
        a: bool,
        #[desc(kind = "II", name = "2nd", description = "second")]
        b: Second::<bool>,
        c: Option<bool>,
        d: (bool, String),
        e: Third,

    }

    #[test]
    fn test() {
        First::field();
    }
}