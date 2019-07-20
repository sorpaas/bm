use bm::{Value, ReadBackend, WriteBackend, ValueOf, Error, Index, DanglingRaw, Leak};
use primitive_types::U256;

use crate::{IntoTree, FromTree, End, Intermediate, CompatibleConstruct};
use crate::utils::{mix_in_type, decode_with_type};

impl IntoTree for bool {
    fn into_tree<DB: WriteBackend>(&self, db: &mut DB) -> Result<ValueOf<DB::Construct>, Error<DB::Error>> where
        DB::Construct: CompatibleConstruct,
    {
        match self {
            true => 1u8.into_tree(db),
            false => 0u8.into_tree(db),
        }
    }
}

impl FromTree for bool {
    fn from_tree<DB: ReadBackend>(root: &ValueOf<DB::Construct>, db: &mut DB) -> Result<Self, Error<DB::Error>> where
        DB::Construct: CompatibleConstruct,
    {
        Ok(u8::from_tree(root, db)? != 0)
    }
}

macro_rules! impl_builtin_uint {
    ( $( $t:ty ),* ) => { $(
        impl IntoTree for $t {
            fn into_tree<DB: WriteBackend>(&self, _db: &mut DB) -> Result<ValueOf<DB::Construct>, Error<DB::Error>> where
                DB::Construct: CompatibleConstruct,
            {
                let mut ret = [0u8; 32];
                let bytes = self.to_le_bytes();
                ret[..bytes.len()].copy_from_slice(&bytes);

                Ok(Value::End(End(ret)))
            }
        }

        impl FromTree for $t {
            fn from_tree<DB: ReadBackend>(root: &ValueOf<DB::Construct>, db: &mut DB) -> Result<Self, Error<DB::Error>> where
                DB::Construct: CompatibleConstruct,
            {
                let raw = DanglingRaw::from_leaked(root.clone());

                match raw.get(db, Index::root())?.ok_or(Error::CorruptedDatabase)? {
                    Value::Intermediate(_) => Err(Error::CorruptedDatabase),
                    Value::End(value) => {
                        let mut bytes = Self::default().to_le_bytes();
                        let bytes_len = bytes.len();
                        bytes.copy_from_slice(&value.0[..bytes_len]);

                        Ok(Self::from_le_bytes(bytes))
                    },
                }
            }
        }
    )* }
}

impl_builtin_uint!(u8, u16, u32, u64, u128);

impl IntoTree for U256 {
    fn into_tree<DB: WriteBackend>(&self, _db: &mut DB) -> Result<ValueOf<DB::Construct>, Error<DB::Error>> where
        DB::Construct: CompatibleConstruct,
    {
        let mut ret = [0u8; 32];
        self.to_little_endian(&mut ret);

        Ok(Value::End(End(ret)))
    }
}

impl FromTree for U256 {
    fn from_tree<DB: ReadBackend>(root: &ValueOf<DB::Construct>, db: &mut DB) -> Result<Self, Error<DB::Error>> where
        DB::Construct: CompatibleConstruct,
    {
        let raw = DanglingRaw::from_leaked(root.clone());

        match raw.get(db, Index::root())?.ok_or(Error::CorruptedDatabase)? {
            Value::Intermediate(_) => Err(Error::CorruptedDatabase),
            Value::End(value) => {
                Ok(U256::from_little_endian(&value.0))
            },
        }
    }
}

impl IntoTree for Value<Intermediate, End> {
    fn into_tree<DB: WriteBackend>(&self, _db: &mut DB) -> Result<ValueOf<DB::Construct>, Error<DB::Error>> where
        DB::Construct: CompatibleConstruct,
    {
        Ok(self.clone())
    }
}

impl FromTree for Value<Intermediate, End> {
    fn from_tree<DB: ReadBackend>(root: &ValueOf<DB::Construct>, _db: &mut DB) -> Result<Self, Error<DB::Error>> where
        DB::Construct: CompatibleConstruct,
    {
        Ok(root.clone())
    }
}

impl<T> FromTree for Option<T> where
    T: FromTree,
{
    fn from_tree<DB: ReadBackend>(root: &ValueOf<DB::Construct>, db: &mut DB) -> Result<Self, Error<DB::Error>> where
        DB::Construct: CompatibleConstruct,
    {
        decode_with_type(root, db, |inner, db, ty| {
            match ty {
                0 => {
                    <()>::from_tree(inner, db)?;
                    Ok(None)
                },
                1 => Ok(Some(T::from_tree(inner, db)?)),
                _ => Err(Error::CorruptedDatabase),
            }
        })
    }
}

impl<T> IntoTree for Option<T> where
    T: IntoTree,
{
    fn into_tree<DB: WriteBackend>(&self, db: &mut DB) -> Result<ValueOf<DB::Construct>, Error<DB::Error>> where
        DB::Construct: CompatibleConstruct,
    {
        match self {
            None => mix_in_type(&(), db, 0),
            Some(value) => mix_in_type(value, db, 1),
        }
    }
}
