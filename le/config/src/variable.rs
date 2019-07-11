/// Implement FromListTreeWithConfig for traits that has already
/// implemented FromListTree and does not need extra configs.
#[macro_export]
macro_rules! impl_from_list_tree_with_empty_config {
    ( $t:ty ) => {
        impl<C, DB> $crate::FromListTreeWithConfig<C, DB> for $t where
            DB: $crate::Backend<Intermediate=Intermediate, End=End>
        {
            fn from_list_tree_with_config(
                root: &$crate::ValueOf<DB>,
                db: &DB,
                max_len: Option<usize>,
                _config: &C,
            ) -> Result<Self, $crate::Error<DB::Error>> {
                <$t>::from_list_tree(root, db, max_len)
            }
        }
    }
}

impl<C, DB, T> FromListTreeWithConfig<C, DB> for ElementalVariableVec<T> where
    ElementalFixedVec<T>: FromVectorTreeWithConfig<C, DB>,
    DB: Backend<Intermediate=Intermediate, End=End>,
{
    fn from_list_tree_with_config(
        root: &ValueOf<DB>,
        db: &DB,
        max_len: Option<usize>,
        config: &C,
    ) -> Result<Self, Error<DB::Error>> {
        from_list_tree(root, db, max_len, |vector_root, db, len, max_len| {
            ElementalFixedVec::<T>::from_vector_tree_with_config(
                &vector_root, db, len, max_len, config
            )
        })
    }
}

/// Traits for list converting from a tree structure with config.
pub trait FromListTreeWithConfig<C, DB: Backend<Intermediate=Intermediate, End=End>>: Sized {
    /// Convert this type from merkle tree, reading nodes from the
    /// given database, with given maximum length.
    fn from_list_tree_with_config(
        root: &ValueOf<DB>,
        db: &DB,
        max_len: Option<usize>,
        config: &C,
    ) -> Result<Self, Error<DB::Error>>;
}

/// Traits for getting the maximum length from config.
pub trait MaxLenFromConfig<C> {
    /// Get the maximum length from config parameter.
    fn max_len_from_config(config: &C) -> Option<usize>;
}

/// Indicate a type that does not have maximum length.
pub struct NoMaxLen;

/// Trait indicate `MaxLenFromConfig` has a known maximum length.
pub trait KnownMaxLen {
    /// Get the static maximum length.
    fn max_len() -> Option<usize>;
}

impl<U: Unsigned> KnownMaxLen for U {
    fn max_len() -> Option<usize> {
        Some(U::to_usize())
    }
}

impl KnownMaxLen for NoMaxLen {
    fn max_len() -> Option<usize> {
        None
    }
}

impl<C, U: KnownMaxLen> MaxLenFromConfig<C> for U {
    fn max_len_from_config(_config: &C) -> Option<usize> {
        U::max_len()
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
/// Variable `Vec` reference.
pub struct VariableVecRef<'a, T, ML>(pub &'a [T], pub Option<usize>, pub PhantomData<ML>);
#[derive(Debug, Clone, Eq, PartialEq)]
/// Variable `Vec` value.
pub struct VariableVec<T, ML>(pub Vec<T>, pub Option<usize>, pub PhantomData<ML>);

impl<'a, T, L> Deref for VariableVecRef<'a, T, L> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        self.0
    }
}

impl<T, L> Deref for VariableVec<T, L> {
    type Target = Vec<T>;

    fn deref(&self) -> &Vec<T> {
        &self.0
    }
}

impl<T, L> DerefMut for VariableVec<T, L> {
    fn deref_mut(&mut self) -> &mut Vec<T> {
        &mut self.0
    }
}

impl<T, ML: KnownMaxLen> Default for VariableVec<T, ML> {
    fn default() -> Self {
        Self(Vec::new(), ML::max_len(), PhantomData)
    }
}

impl<C, T, ML: MaxLenFromConfig<C>> DefaultWithConfig<C> for VariableVec<T, ML> {
    fn default_with_config(config: &C) -> Self {
        Self(Vec::new(), ML::max_len_from_config(config), PhantomData)
    }
}

impl<'a, T, ML> Composite for VariableVecRef<'a, T, ML> { }
impl<T, ML> Composite for VariableVec<T, ML> { }

impl<'a, DB, T, L> IntoTree<DB> for VariableVecRef<'a, T, L> where
    for<'b> ElementalVariableVecRef<'b, T>: IntoListTree<DB>,
    DB: Backend<Intermediate=Intermediate, End=End>,
{
    fn into_tree(&self, db: &mut DB) -> Result<ValueOf<DB>, Error<DB::Error>> {
        ElementalVariableVecRef(self.0).into_list_tree(db, self.1)
    }
}

impl<DB, T, L> IntoTree<DB> for VariableVec<T, L> where
    for<'b> ElementalVariableVecRef<'b, T>: IntoListTree<DB>,
    DB: Backend<Intermediate=Intermediate, End=End>,
{
    fn into_tree(&self, db: &mut DB) -> Result<ValueOf<DB>, Error<DB::Error>> {
        ElementalVariableVecRef(&self.0).into_list_tree(db, self.1)
    }
}

impl<DB, T, ML: KnownMaxLen> FromTree<DB> for VariableVec<T, ML> where
    for<'a> ElementalVariableVec<T>: FromListTree<DB>,
    DB: Backend<Intermediate=Intermediate, End=End>,
{
    fn from_tree(root: &ValueOf<DB>, db: &DB) -> Result<Self, Error<DB::Error>> {
        let value = ElementalVariableVec::<T>::from_list_tree(root, db, ML::max_len())?;
        Ok(VariableVec(value.0, ML::max_len(), PhantomData))
    }
}

impl<DB, C, T, ML: MaxLenFromConfig<C>> FromTreeWithConfig<C, DB> for VariableVec<T, ML> where
    for<'a> ElementalVariableVec<T>: FromListTree<DB>,
    DB: Backend<Intermediate=Intermediate, End=End>,
{
    fn from_tree_with_config(
        root: &ValueOf<DB>,
        db: &DB,
        config: &C
    ) -> Result<Self, Error<DB::Error>> {
        let max_len = ML::max_len_from_config(config);
        let value = ElementalVariableVec::<T>::from_list_tree(
            root,
            db,
            max_len,
        )?;
        Ok(VariableVec(value.0, max_len, PhantomData))
    }
}
