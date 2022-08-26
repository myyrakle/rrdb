macro_rules! data_types {
	($($name:ident = $oid:expr, $size: expr)*) => {
		#[derive(Debug, Copy, Clone)]
		/// Describes a Postgres data type.
		pub enum DataTypeOid {
			$(
				#[allow(missing_docs)]
				$name,
			)*
			/// A type which is not known to this crate.
			Unknown(u32),
		}

		impl DataTypeOid {
			/// Fetch the size in bytes for this data type.
			/// Variably-sized types return -1.
			pub fn size_bytes(&self) -> i16 {
				match self {
					$(
						Self::$name => $size,
					)*
					Self::Unknown(_) => unimplemented!(),
				}
			}
		}

		impl From<u32> for DataTypeOid {
			fn from(value: u32) -> Self {
				match value {
					$(
						$oid => Self::$name,
					)*
					other => Self::Unknown(other),
				}
			}
		}

		impl From<DataTypeOid> for u32 {
			fn from(value: DataTypeOid) -> Self {
				match value {
					$(
						DataTypeOid::$name => $oid,
					)*
					DataTypeOid::Unknown(other) => other,
				}
			}
		}
	};
}

// For oid see:
// https://github.com/sfackler/rust-postgres/blob/master/postgres-types/src/type_gen.rs
data_types! {
    Unspecified = 0, 0

    Bool = 16, 1

    Int2 = 21, 2
    Int4 = 23, 4
    Int8 = 20, 8

    Float4 = 700, 4
    Float8 = 701, 8

    Date = 1082, 4
    Timestamp = 1114, 8

    Text = 25, -1
}
