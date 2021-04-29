use super::{
    AsValue, ConcreteValueType, IrTypeContext, IrValueContext, PointerValueType, SizedValueType,
    Value, ValueType,
};
use crate::value::AddressableType;
use inkwell::{
    module::Linkage,
    types::PointerType,
    values::{BasicValueEnum, UnnamedAddress},
    AddressSpace,
};
use std::marker::PhantomData;

/// Represents a typed global value. A `Global<T>` can be constructed from any `Value<T>` that can
/// be converted to a [`inkwell::values::BasicValueEnum`].
///
/// Globals can be used to store data inside an inkwell context which can be referenced from code.
///
/// Like `Value<T>` a `Global<T>` is typed on the type of data that it stores.
pub struct Global<'ink, T: ?Sized> {
    pub value: inkwell::values::GlobalValue<'ink>,
    data: PhantomData<T>,
}

impl<'ink, T: ?Sized> Clone for Global<'ink, T> {
    fn clone(&self) -> Self {
        Global {
            value: self.value,
            data: self.data,
        }
    }
}

impl<'ink, T: ?Sized> Copy for Global<'ink, T> {}

impl<'ink, T: ?Sized> Global<'ink, T> {
    /// Creates a `Global<T>` from an underlying value.
    ///
    /// # Safety
    ///
    /// There is no guarantee that the passed value actually represents the type `T`. Sometimes this
    /// can however be very useful. This method is marked as unsafe since there is also no way to
    /// check the correctness.
    pub unsafe fn from_raw(value: inkwell::values::GlobalValue<'ink>) -> Self {
        Global {
            value,
            data: Default::default(),
        }
    }
}

impl<'ink, T: ?Sized> ConcreteValueType<'ink> for *const Global<'ink, T> {
    type Value = inkwell::values::PointerValue<'ink>;
}

impl<'ink, T: PointerValueType<'ink> + ?Sized> SizedValueType<'ink> for *const Global<'ink, T> {
    fn get_ir_type(context: &IrTypeContext<'ink, '_>) -> <Self::Value as ValueType<'ink>>::Type {
        T::get_ptr_type(context, None)
    }
}

impl<'ink, T: PointerValueType<'ink> + ?Sized> PointerValueType<'ink> for *const Global<'ink, T> {
    fn get_ptr_type(
        context: &IrTypeContext<'ink, '_>,
        address_space: Option<AddressSpace>,
    ) -> PointerType<'ink> {
        debug_assert!(
            address_space.is_none() || address_space == Some(AddressSpace::Generic),
            "Globals can only live in generic address space"
        );
        T::get_ptr_type(context, None)
    }
}

impl<'ink, T: ?Sized, I> AsValue<'ink, *const I> for Global<'ink, T>
where
    *const I: ConcreteValueType<'ink, Value = inkwell::values::PointerValue<'ink>>,
    T: AddressableType<'ink, I>,
{
    fn as_value(&self, context: &IrValueContext<'ink, '_, '_>) -> Value<'ink, *const I> {
        Value::from_raw(T::ptr_cast(self.value.as_pointer_value(), context))
    }
}

impl<'ink, T: ConcreteValueType<'ink> + ?Sized> Value<'ink, T>
where
    T::Value: Into<BasicValueEnum<'ink>>,
{
    pub fn into_global<S: AsRef<str>>(
        self,
        name: S,
        context: &IrValueContext<'ink, '_, '_>,
        is_const: bool,
        linkage: Linkage,
        unnamed_addr: Option<UnnamedAddress>,
    ) -> Global<'ink, T> {
        // NOTE: No support for address spaces
        let address_space = None;

        let initializer = self.value.into();
        let global =
            context
                .module
                .add_global(initializer.get_type(), address_space, name.as_ref());
        global.set_linkage(linkage);
        global.set_constant(is_const);
        global.set_initializer(&initializer);
        if let Some(addr) = unnamed_addr {
            global.set_unnamed_address(addr);
        }
        Global {
            value: global,
            data: Default::default(),
        }
    }

    /// Converts self into a private const global. A private const global always has Private linkage
    /// so its only accessible from the module it's defined in. Its address is globally
    /// insignificant because the linker can rename it.
    ///
    /// This is useful for constant values that require dynamic sizing like arrays or strings but
    /// still need to be referenced in the code. We can't use const arrays here because the size
    /// must be constant in the type.
    ///
    /// e.g. in the following case:
    /// ```c
    /// const char str[] = "foobar"
    /// ```
    ///
    /// The type of `str` is a 'dynamically' sized array which makes the type `const char*`. Not,
    /// `const char[6]`. We use a const private to store the array and create a pointer from that
    /// value.
    pub fn into_const_private_global<S: AsRef<str>>(
        self,
        name: S,
        context: &IrValueContext<'ink, '_, '_>,
    ) -> Global<'ink, T> {
        self.into_global(
            name,
            context,
            true,
            Linkage::Private,
            Some(UnnamedAddress::Global),
        )
    }
}

impl<'ink, T: ?Sized> From<Global<'ink, T>> for inkwell::values::PointerValue<'ink> {
    fn from(global: Global<'ink, T>) -> Self {
        global.value.as_pointer_value()
    }
}
