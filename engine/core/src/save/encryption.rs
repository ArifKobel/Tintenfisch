use crate::crypt::SecurityHandler;
use crate::syntax::Dict;

#[derive(Clone, Debug)]
pub enum Encryption {
    Source,
    Sealed { dictionary: Dict, handler: SecurityHandler },
    Plain,
}
