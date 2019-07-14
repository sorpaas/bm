use bm::{Backend, ValueOf, Error, Value, DanglingVector, Leak};
use bm::utils::vector_tree;
use primitive_types::{H256, H512};
use generic_array::{GenericArray, ArrayLength};
use crate::{ElementalFixedVecRef, ElementalFixedVec, IntoCompositeVectorTree,
            IntoCompactVectorTree, IntoTree, FromTree, FromCompositeVectorTree,
            FromCompactVectorTree, Intermediate, End, Compact, CompactRef};

impl<'a, T, L: ArrayLength<T>, DB> IntoTree<DB> for CompactRef<'a, GenericArray<T, L>> where
    DB: Backend<Intermediate=Intermediate, End=End>,
    for<'b> ElementalFixedVecRef<'b, T>: IntoCompactVectorTree<DB>,
{
    fn into_tree(&self, db: &mut DB) -> Result<ValueOf<DB>, Error<DB::Error>> {
        ElementalFixedVecRef(&self.0).into_compact_vector_tree(db, None)
    }
}

impl<T, L: ArrayLength<T>, DB> IntoTree<DB> for Compact<GenericArray<T, L>> where
    DB: Backend<Intermediate=Intermediate, End=End>,
    for<'a> ElementalFixedVecRef<'a, T>: IntoCompactVectorTree<DB>,
{
    fn into_tree(&self, db: &mut DB) -> Result<ValueOf<DB>, Error<DB::Error>> {
        ElementalFixedVecRef(&self.0).into_compact_vector_tree(db, None)
    }
}

impl<T, L: ArrayLength<T>, DB> FromTree<DB> for Compact<GenericArray<T, L>> where
    DB: Backend<Intermediate=Intermediate, End=End>,
    T: Default,
    ElementalFixedVec<T>: FromCompactVectorTree<DB>,
{
    fn from_tree(root: &ValueOf<DB>, db: &DB) -> Result<Self, Error<DB::Error>> {
        let value = ElementalFixedVec::<T>::from_compact_vector_tree(root, db, L::to_usize(), None)?;
        let mut ret = GenericArray::default();
        for (i, v) in value.0.into_iter().enumerate() {
            ret[i] = v;
        }
        Ok(Self(ret))
    }
}

impl<DB> IntoTree<DB> for H256 where
    DB: Backend<Intermediate=Intermediate, End=End>
{
    fn into_tree(&self, db: &mut DB) -> Result<ValueOf<DB>, Error<DB::Error>> {
        ElementalFixedVecRef(&self.0.as_ref()).into_compact_vector_tree(db, None)
    }
}

impl<DB> FromTree<DB> for H256 where
    DB: Backend<Intermediate=Intermediate, End=End>
{
    fn from_tree(root: &ValueOf<DB>, db: &DB) -> Result<Self, Error<DB::Error>> {
        let value = ElementalFixedVec::<u8>::from_compact_vector_tree(root, db, 32, None)?;
        Ok(Self::from_slice(value.0.as_ref()))
    }
}

impl<DB> IntoTree<DB> for H512 where
    DB: Backend<Intermediate=Intermediate, End=End>
{
    fn into_tree(&self, db: &mut DB) -> Result<ValueOf<DB>, Error<DB::Error>> {
        ElementalFixedVecRef(&self.0.as_ref()).into_compact_vector_tree(db, None)
    }
}

impl<DB> FromTree<DB> for H512 where
    DB: Backend<Intermediate=Intermediate, End=End>
{
    fn from_tree(root: &ValueOf<DB>, db: &DB) -> Result<Self, Error<DB::Error>> {
        let value = ElementalFixedVec::<u8>::from_compact_vector_tree(root, db, 32, None)?;
        Ok(Self::from_slice(value.0.as_ref()))
    }
}

macro_rules! impl_fixed_array {
    ( $( $n:expr ),* ) => { $(
        impl<DB, T> IntoTree<DB> for [T; $n] where
            DB: Backend<Intermediate=Intermediate, End=End>,
            for<'a> ElementalFixedVecRef<'a, T>: IntoCompositeVectorTree<DB>,
        {
            fn into_tree(&self, db: &mut DB) -> Result<ValueOf<DB>, Error<DB::Error>> {
                ElementalFixedVecRef(&self[..]).into_composite_vector_tree(db, None)
            }
        }

        impl<DB, T> FromTree<DB> for [T; $n] where
            DB: Backend<Intermediate=Intermediate, End=End>,
            T: Default + Copy,
            for<'a> ElementalFixedVec<T>: FromCompositeVectorTree<DB>,
        {
            fn from_tree(root: &ValueOf<DB>, db: &DB) -> Result<Self, Error<DB::Error>> {
                let value = ElementalFixedVec::<T>::from_composite_vector_tree(root, db, $n, None)?;
                let mut ret = [T::default(); $n];
                for (i, v) in value.0.into_iter().enumerate() {
                    ret[i] = v;
                }
                Ok(ret)
            }
        }
    )* }
}

impl_fixed_array!(1, 2, 3, 4, 5, 6, 7, 8,
                  9, 10, 11, 12, 13, 14, 15, 16,
                  17, 18, 19, 20, 21, 22, 23, 24,
                  25, 26, 27, 28, 29, 30, 31, 32);

impl<DB, T, L: ArrayLength<T>> IntoTree<DB> for GenericArray<T, L> where
    DB: Backend<Intermediate=Intermediate, End=End>,
    for<'a> ElementalFixedVecRef<'a, T>: IntoCompositeVectorTree<DB>,
{
    fn into_tree(&self, db: &mut DB) -> Result<ValueOf<DB>, Error<DB::Error>> {
        ElementalFixedVecRef(&self[..]).into_composite_vector_tree(db, None)
    }
}

impl<DB, T, L: ArrayLength<T>> FromTree<DB> for GenericArray<T, L> where
    DB: Backend<Intermediate=Intermediate, End=End>,
    for<'a> ElementalFixedVec<T>: FromCompositeVectorTree<DB>,
{
    fn from_tree(root: &ValueOf<DB>, db: &DB) -> Result<Self, Error<DB::Error>> {
        let value = ElementalFixedVec::<T>::from_composite_vector_tree(root, db, L::to_usize(), None)?;
        Ok(GenericArray::from_exact_iter(value.0)
           .expect("Fixed vec must build vector with L::as_usize; qed"))
    }
}

impl<DB> FromTree<DB> for () where
    DB: Backend<Intermediate=Intermediate, End=End>
{
    fn from_tree(root: &ValueOf<DB>, _db: &DB) -> Result<Self, Error<DB::Error>> {
        if root == &Value::End(Default::default()) {
            Ok(())
        } else {
            Err(Error::CorruptedDatabase)
        }
    }
}

impl<DB> IntoTree<DB> for () where
    DB: Backend<Intermediate=Intermediate, End=End>
{
    fn into_tree(&self, _db: &mut DB) -> Result<ValueOf<DB>, Error<DB::Error>> {
        Ok(Value::End(Default::default()))
    }
}

macro_rules! impl_tuple {
    ($len:expr, $($i:ident => $t:ident),+) => {
        impl<DB, $($t: FromTree<DB>),+> FromTree<DB> for ($($t,)+) where
            DB: Backend<Intermediate=Intermediate, End=End>
        {
            fn from_tree(root: &ValueOf<DB>, db: &DB) -> Result<Self, Error<DB::Error>> {
                let vector = DanglingVector::<DB>::from_leaked(
                    (root.clone(), $len, None)
                );
                let mut i = 0;
                Ok(($({
                    let value = <$t>::from_tree(&vector.get(db, i)?, db)?;
                    #[allow(unused_assignments)] {
                        i += 1;
                    }
                    value
                }),+))
            }
        }

        impl<DB, $($t: IntoTree<DB>),+> IntoTree<DB> for ($($t),+) where
            DB: Backend<Intermediate=Intermediate, End=End>
        {
            fn into_tree(&self, db: &mut DB) -> Result<ValueOf<DB>, Error<DB::Error>> {
                let ($($i),+) = self;
                let mut vector = Vec::new();
                $({
                    vector.push($i.into_tree(db)?);
                })+
                vector_tree(&vector, db, None)
            }
        }
    }
}

impl_tuple!(2, a => A, b => B);
impl_tuple!(3, a => A, b => B, c => C);
impl_tuple!(4, a => A, b => B, c => C, d => D);
impl_tuple!(5, a => A, b => B, c => C, d => D, e => E);
impl_tuple!(6, a => A, b => B, c => C, d => D, e => E, f => F);
impl_tuple!(7, a => A, b => B, c => C, d => D, e => E, f => F, g => G);
impl_tuple!(8, a => A, b => B, c => C, d => D, e => E, f => F, g => G, h => H);
impl_tuple!(9, a => A, b => B, c => C, d => D, e => E, f => F, g => G, h => H, i => I);
