use core::fmt;
use core::marker::PhantomData;
use core::ops::Deref;

pub struct ReadOnly<'a, T> {
    value: T,
    owner: PhantomData<&'a ()>,
}

impl<T> ReadOnly<'_, T> {
    pub(crate) const fn new(value: T) -> Self {
        Self {
            value,
            owner: PhantomData,
        }
    }
}

impl<T> Deref for ReadOnly<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.value
    }
}

impl<T: fmt::Debug> fmt::Debug for ReadOnly<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.value.fmt(f)
    }
}
