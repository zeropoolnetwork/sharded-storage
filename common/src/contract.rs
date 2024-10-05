#[derive(Debug)]
enum ContractError {
    StorageError,
    ExecutionError,
}

impl std::fmt::Display for ContractError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ContractError::StorageError => write!(f, "Storage error"),
            ContractError::ExecutionError => write!(f, "Execution error"),
        }
    }
}

impl std::error::Error for ContractError {}

pub trait SmartContract {
    fn reserve_storage(
        &self,
        sectors: &[usize],
        pk: &str, // FIXME
    ) -> impl std::future::Future<Output = Result<(), ContractError>> + Send;

    fn commit_proof(
        &self,
        sector: usize,
        proof: &str, // FIXME
    ) -> impl std::future::Future<Output = Result<(), ContractError>> + Send;
}
