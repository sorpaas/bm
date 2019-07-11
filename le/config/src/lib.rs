/// Traits for type converting from a tree structure with a config.
pub trait FromTreeWithConfig<C, DB: Backend<Intermediate=Intermediate, End=End>>: Sized {
    /// Convert this type from merkle tree, reading nodes from the
    /// given database, with the given config.
    fn from_tree_with_config(root: &ValueOf<DB>, db: &DB, config: &C) -> Result<Self, Error<DB::Error>>;
}

/// Traits for getting default value from a config.
pub trait DefaultWithConfig<C>: Sized {
    /// Get the default value.
    fn default_with_config(config: &C) -> Self;
}

/// Implement FromTreeWithConfig for traits that has already
/// implemented FromTree and does not need extra configs.
#[macro_export]
macro_rules! impl_from_tree_with_empty_config {
    ( $t:ty ) => {
        impl<DB, C> $crate::FromTreeWithConfig<C, DB> for $t where
            DB: $crate::Backend<Intermediate=$crate::Intermediate, End=$crate::End>,
        {
            fn from_tree_with_config(
                root: &$crate::ValueOf<DB>,
                db: &DB,
                _config: &C
            ) -> Result<Self, $crate::Error<DB::Error>> {
                $crate::FromTree::from_tree(root, db)
            }
        }
    }
}

macro_rules! impl_from_tree_with_empty_config_for_builtin_uint {
    ( $( $t:ty ),* ) => { $(
        impl_from_tree_with_empty_config!($t);
    )* }
}

impl_from_tree_with_empty_config!(bool);
impl_from_tree_with_empty_config!(u8);
impl_from_tree_with_empty_config!(u16);
impl_from_tree_with_empty_config!(u32);
impl_from_tree_with_empty_config!(u64);
impl_from_tree_with_empty_config!(u128);
impl_from_tree_with_empty_config!(U256);
impl_from_tree_with_empty_config!(H256);

impl<DB, T, L: ArrayLength<T>, C> FromTreeWithConfig<C, DB> for GenericArray<T, L> where
    DB: Backend<Intermediate=Intermediate, End=End>,
    for<'a> ElementalFixedVec<T>: FromVectorTree<DB>,
{
    fn from_tree_with_config(
        root: &ValueOf<DB>,
        db: &DB,
        _config: &C
    ) -> Result<Self, Error<DB::Error>> {
        FromTree::from_tree(root, db)
    }
}

macro_rules! impl_from_tree_with_empty_config_for_fixed_array {
    ( $( $n:expr ),* ) => { $(
        impl<DB, T, C> FromTreeWithConfig<C, DB> for [T; $n] where
            DB: Backend<Intermediate=Intermediate, End=End>,
            T: Default + Copy,
            for<'a> ElementalFixedVec<T>: FromVectorTree<DB>,
        {
            fn from_tree_with_config(
                root: &ValueOf<DB>,
                db: &DB,
                _config: &C
            ) -> Result<Self, Error<DB::Error>> {
                FromTree::from_tree(root, db)
            }
        }
    ) }
}

impl_from_tree_with_empty_config_for_fixed_array!(
    1, 2, 3, 4, 5, 6, 7, 8,
    9, 10, 11, 12, 13, 14, 15, 16,
    17, 18, 19, 20, 21, 22, 23, 24,
    25, 26, 27, 28, 29, 30, 31, 32
);
