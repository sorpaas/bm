/// Traits for vector converting from a tree structure with config.
pub trait FromVectorTreeWithConfig<C, DB: Backend<Intermediate=Intermediate, End=End>>: Sized {
    /// Convert this type from merkle tree, reading nodes from the
    /// given database, with given length and maximum length.
    fn from_vector_tree_with_config(
        root: &ValueOf<DB>,
        db: &DB,
        len: usize,
        max_len: Option<usize>,
        config: &C,
    ) -> Result<Self, Error<DB::Error>>;
}

/// Implement FromVectorTreeWithConfig for traits that has already
/// implemented FromVectorTree and does not need extra configs.
#[macro_export]
macro_rules! impl_from_vector_tree_with_empty_config {
    ( $t:ty ) => {
        impl<C, DB> $crate::FromVectorTreeWithConfig<C, DB> for $t where
            DB: $crate::Backend<Intermediate=Intermediate, End=End>
        {
            fn from_vector_tree_with_config(
                root: &$crate::ValueOf<DB>,
                db: &DB,
                len: usize,
                max_len: Option<usize>,
                _config: &C,
            ) -> Result<Self, $crate::Error<DB::Error>> {
                <$t>::from_vector_tree(root, db, len, max_len)
            }
        }
    }
}

impl_from_vector_tree_with_empty_config!(ElementalFixedVec<u8>);
impl_from_vector_tree_with_empty_config!(ElementalFixedVec<u16>);
impl_from_vector_tree_with_empty_config!(ElementalFixedVec<u32>);
impl_from_vector_tree_with_empty_config!(ElementalFixedVec<u64>);
impl_from_vector_tree_with_empty_config!(ElementalFixedVec<u128>);
impl_from_vector_tree_with_empty_config!(ElementalFixedVec<U256>);
impl_from_vector_tree_with_empty_config!(ElementalFixedVec<bool>);


impl<DB, C, T: Composite + FromTreeWithConfig<C, DB>> FromVectorTreeWithConfig<C, DB> for ElementalFixedVec<T> where
    DB: Backend<Intermediate=Intermediate, End=End>
{
    fn from_vector_tree_with_config(
        root: &ValueOf<DB>,
        db: &DB,
        len: usize,
        max_len: Option<usize>,
        config: &C,
    ) -> Result<Self, Error<DB::Error>> {
        from_vector_tree(root, db, len, max_len, |value, db| {
            T::from_tree_with_config(value, db, config)
        })
    }
}

/// Traits for getting the length from config.
pub trait LenFromConfig<C> {
    /// Get the length from config parameter.
    fn len_from_config(config: &C) -> usize;
}

/// Trait indicate `LenFromConfig` has a known maximum length.
pub trait KnownLen {
    /// Get the static length.
    fn len() -> usize;
}

impl<U: Unsigned> KnownLen for U {
    fn len() -> usize {
        U::to_usize()
    }
}

impl<C, U: KnownLen> LenFromConfig<C> for U {
    fn len_from_config(_config: &C) -> usize {
        U::len()
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
/// Fixed `Vec` reference. In ssz's definition, this is a "vector".
pub struct FixedVecRef<'a, T, L>(pub &'a [T], pub PhantomData<L>);
#[derive(Debug, Clone, Eq, PartialEq)]
/// Fixed `Vec` value. In ssz's definition, this is a "vector".
pub struct FixedVec<T, L>(pub Vec<T>, pub PhantomData<L>);

impl<'a, T, L> Deref for FixedVecRef<'a, T, L> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        self.0
    }
}

impl<T, L> Deref for FixedVec<T, L> {
    type Target = Vec<T>;

    fn deref(&self) -> &Vec<T> {
        &self.0
    }
}

impl<T, L> DerefMut for FixedVec<T, L> {
    fn deref_mut(&mut self) -> &mut Vec<T> {
        &mut self.0
    }
}

impl<T: Default, L: KnownLen> Default for FixedVec<T, L> {
    fn default() -> Self {
        let len = L::len();
        let mut ret = Vec::new();
        for _ in 0..len {
            ret.push(T::default());
        }
        Self(ret, PhantomData)
    }
}

impl<C, T: Default, L: LenFromConfig<C>> DefaultWithConfig<C> for FixedVec<T, L> {
    fn default_with_config(config: &C) -> Self {
        let len = L::len_from_config(config);
        let mut ret = Vec::new();
        for _ in 0..len {
            ret.push(T::default());
        }
        Self(ret, PhantomData)
    }
}

impl<'a, T, L> Composite for FixedVecRef<'a, T, L> { }
impl<T, L> Composite for FixedVec<T, L> { }

impl<'a, DB, T, L> IntoTree<DB> for FixedVecRef<'a, T, L> where
    for<'b> ElementalFixedVecRef<'b, T>: IntoVectorTree<DB>,
    DB: Backend<Intermediate=Intermediate, End=End>,
{
    fn into_tree(&self, db: &mut DB) -> Result<ValueOf<DB>, Error<DB::Error>> {
        ElementalFixedVecRef(self.0).into_vector_tree(db, None)
    }
}

impl<DB, T, L> IntoTree<DB> for FixedVec<T, L> where
    for<'b> ElementalFixedVecRef<'b, T>: IntoVectorTree<DB>,
    DB: Backend<Intermediate=Intermediate, End=End>,
{
    fn into_tree(&self, db: &mut DB) -> Result<ValueOf<DB>, Error<DB::Error>> {
        ElementalFixedVecRef(&self.0).into_vector_tree(db, None)
    }
}

impl<DB, T, L: Unsigned> FromTree<DB> for FixedVec<T, L> where
    for<'a> ElementalFixedVec<T>: FromVectorTree<DB>,
    DB: Backend<Intermediate=Intermediate, End=End>,
{
    fn from_tree(root: &ValueOf<DB>, db: &DB) -> Result<Self, Error<DB::Error>> {
        let value = ElementalFixedVec::<T>::from_vector_tree(root, db, L::to_usize(), None)?;
        Ok(FixedVec(value.0, PhantomData))
    }
}

impl<DB, C, T, L: LenFromConfig<C>> FromTreeWithConfig<C, DB> for FixedVec<T, L> where
    for<'a> ElementalFixedVec<T>: FromVectorTree<DB>,
    DB: Backend<Intermediate=Intermediate, End=End>,
{
    fn from_tree_with_config(
        root: &ValueOf<DB>,
        db: &DB,
        config: &C
    ) -> Result<Self, Error<DB::Error>> {
        let value = ElementalFixedVec::<T>::from_vector_tree(
            root,
            db,
            L::len_from_config(config),
            None
        )?;
        Ok(FixedVec(value.0, PhantomData))
    }
}
