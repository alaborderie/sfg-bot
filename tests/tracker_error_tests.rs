use sfg_bot::RepositoryError;
use sfg_bot::riot::tracker::TrackerError;

mod tracker_error {
    use super::*;

    #[test]
    fn tracker_error_is_debug() {
        let repo_err = RepositoryError::Database(sqlx::Error::RowNotFound);
        let err = TrackerError::Database(repo_err);
        let debug = format!("{:?}", err);
        assert!(debug.contains("Database"));
    }

    #[test]
    fn database_error_display() {
        let repo_err = RepositoryError::Database(sqlx::Error::RowNotFound);
        let err = TrackerError::Database(repo_err);
        let display = format!("{}", err);
        assert!(display.contains("Database error"));
    }
}
