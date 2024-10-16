use crate::error::Error;
use std::convert::TryFrom;
use termcolor::{Color, ColorSpec};

#[derive(Debug)]
pub(crate) enum StackStatus {
    CreateInProgress,
    CreateComplete,
    CreateFailed,
    DeleteComplete,
    DeleteFailed,
    DeleteSkipped,
    DeleteInProgress,
    ReviewInProgress,
    RollbackComplete,
    RollbackFailed,
    RollbackInProgress,
    UpdateComplete,
    UpdateCompleteCleanupInProgress,
    UpdateFailed,
    UpdateInProgress,
    UpdateRollbackComplete,
    UpdateRollbackCompleteCleanupInProgress,
    UpdateRollbackFailed,
    UpdateRollbackInProgress,
    ImportInProgress,
    ImportComplete,
    ImportRollbackInProgress,
    ImportRollbackFailed,
    ImportRollbackComplete,
}

impl TryFrom<&str> for StackStatus {
    type Error = Error<()>;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        use StackStatus::*;
        match value {
            "CREATE_IN_PROGRESS" => Ok(CreateInProgress),
            "CREATE_COMPLETE" => Ok(CreateComplete),
            "CREATE_FAILED" => Ok(CreateFailed),
            "DELETE_COMPLETE" => Ok(DeleteComplete),
            "DELETE_FAILED" => Ok(DeleteFailed),
            "DELETE_IN_PROGRESS" => Ok(DeleteInProgress),
            "REVIEW_IN_PROGRESS" => Ok(ReviewInProgress),
            "ROLLBACK_COMPLETE" => Ok(RollbackComplete),
            "ROLLBACK_FAILED" => Ok(RollbackFailed),
            "ROLLBACK_IN_PROGRESS" => Ok(RollbackInProgress),
            "UPDATE_COMPLETE" => Ok(UpdateComplete),
            "UPDATE_COMPLETE_CLEANUP_IN_PROGRESS" => Ok(UpdateCompleteCleanupInProgress),
            "UPDATE_FAILED" => Ok(UpdateFailed),
            "UPDATE_IN_PROGRESS" => Ok(UpdateInProgress),
            "UPDATE_ROLLBACK_COMPLETE" => Ok(UpdateRollbackComplete),
            "UPDATE_ROLLBACK_COMPLETE_CLEANUP_IN_PROGRESS" => {
                Ok(UpdateRollbackCompleteCleanupInProgress)
            }
            "UPDATE_ROLLBACK_FAILED" => Ok(UpdateRollbackFailed),
            "UPDATE_ROLLBACK_IN_PROGRESS" => Ok(UpdateRollbackInProgress),
            "IMPORT_IN_PROGRESS" => Ok(ImportInProgress),
            "IMPORT_COMPLETE" => Ok(ImportComplete),
            "IMPORT_ROLLBACK_IN_PROGRESS" => Ok(ImportRollbackInProgress),
            "IMPORT_ROLLBACK_FAILED" => Ok(ImportRollbackFailed),
            "IMPORT_ROLLBACK_COMPLETE" => Ok(ImportRollbackComplete),
            "DELETE_SKIPPED" => Ok(DeleteSkipped),
            _ => unreachable!("{}", value),
        }
    }
}

impl StackStatus {
    pub(crate) fn color_spec(&self) -> Option<ColorSpec> {
        let mut spec = ColorSpec::new();
        match self {
            Self::CreateInProgress
            | Self::DeleteInProgress
            | Self::DeleteSkipped
            | Self::ReviewInProgress
            | Self::RollbackInProgress
            | Self::UpdateInProgress
            | Self::UpdateCompleteCleanupInProgress
            | Self::UpdateRollbackCompleteCleanupInProgress
            | Self::ImportRollbackInProgress
            | Self::UpdateRollbackInProgress
            | Self::ImportInProgress => {
                spec.set_fg(Some(Color::Blue));
            }

            Self::CreateComplete
            | Self::DeleteComplete
            | Self::RollbackComplete
            | Self::UpdateComplete
            | Self::UpdateRollbackComplete
            | Self::ImportComplete
            | Self::ImportRollbackComplete => {
                spec.set_fg(Some(Color::Green));
            }

            Self::CreateFailed
            | Self::DeleteFailed
            | Self::RollbackFailed
            | Self::UpdateFailed
            | Self::UpdateRollbackFailed
            | Self::ImportRollbackFailed => {
                spec.set_fg(Some(Color::Red));
            }
        };
        Some(spec)
    }

    pub(crate) fn is_complete(&self) -> bool {
        matches!(
            self,
            Self::CreateComplete
                | Self::DeleteComplete
                | Self::RollbackComplete
                | Self::UpdateComplete
                | Self::UpdateRollbackComplete
                | Self::ImportComplete
                | Self::ImportRollbackComplete
        )
    }
}
