use bevy_reflect::{Enum, Reflect, Struct, Tuple, TupleStruct};

/// A generic trait that represents the field access ability of several traits from `bevy_reflect`.
/// Should not need to be implemented or used by user types.
pub trait FieldAccess {
    /// Get the number of fields.
    fn field_len(&self) -> usize;

    /// Get the nth field.
    fn field(&mut self, index: usize) -> &mut dyn Reflect;

    /// Get the name of the nth field.
    fn name(&self, index: usize) -> Option<&str>;

    /// Get the type name of the implementor.
    fn type_name(&self) -> &str;
}

impl FieldAccess for &mut dyn Struct {
    fn field_len(&self) -> usize {
        Struct::field_len(*self)
    }

    fn field(&mut self, index: usize) -> &mut dyn Reflect {
        self.field_at_mut(index).unwrap()
    }

    fn name(&self, index: usize) -> Option<&str> {
        Some(self.name_at(index).unwrap())
    }

    fn type_name(&self) -> &str {
        <dyn Struct>::type_name(*self)
    }
}

impl FieldAccess for &mut dyn TupleStruct {
    fn field_len(&self) -> usize {
        TupleStruct::field_len(*self)
    }

    fn field(&mut self, index: usize) -> &mut dyn Reflect {
        self.field_mut(index).unwrap()
    }

    fn name(&self, _: usize) -> Option<&str> {
        None
    }

    fn type_name(&self) -> &str {
        <dyn TupleStruct>::type_name(*self)
    }
}

impl FieldAccess for &mut dyn Tuple {
    fn field_len(&self) -> usize {
        Tuple::field_len(*self)
    }

    fn field(&mut self, index: usize) -> &mut dyn Reflect {
        self.field_mut(index).unwrap()
    }

    fn name(&self, _: usize) -> Option<&str> {
        None
    }

    fn type_name(&self) -> &str {
        <dyn Tuple>::type_name(*self)
    }
}

impl FieldAccess for &mut dyn Enum {
    fn field_len(&self) -> usize {
        Enum::field_len(*self)
    }

    fn field(&mut self, index: usize) -> &mut dyn Reflect {
        self.field_at_mut(index).unwrap()
    }

    fn name(&self, index: usize) -> Option<&str> {
        self.name_at(index)
    }

    fn type_name(&self) -> &str {
        <dyn Enum>::type_name(*self)
    }
}