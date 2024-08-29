mod conditions;
mod matcher;

use notify::{Event, EventKind};
use serde::Deserialize;
use std::io::Error;
use std::path::{Path, PathBuf};
use tokio::{fs, io::AsyncReadExt, process::Command};
use tracing::{error, trace};

use conditions::{CheckArgs, ConditionChecker, ConditionOrConditionsGroup};

#[derive(Debug, Deserialize, Clone)]
pub struct Action {
    triggers: Vec<EventKind>, // События файловой системы, на которые реагирует действие
    conditions: ConditionOrConditionsGroup, // Условия для выполнения действия
    action_type: ActionType,  // Тип действия
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum ActionType {
    MoveFile(MoveFileAction),
    DeleteFile(DeleteFileAction),
    CreateSymlink(CreateSymlinkAction),
    Custom(CustomAction),
}

#[derive(Debug, Deserialize, Clone)]
pub struct MoveFileAction {
    destination: PathBuf,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DeleteFileAction {
    force: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CreateSymlinkAction {
    to: PathBuf,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CustomAction {
    command: String,
}
impl CustomAction {
    pub async fn execute_command(&self, path: &Path) -> Result<(), Error> {
        let command_with_path = self.command.replace("{}", &path.to_string_lossy());
        let output = Command::new("sh")
            .arg("-c")
            .arg(command_with_path)
            .output()
            .await?;

        if !output.status.success() {
            error!("Command failed with status: {:?}", output.status);
            error!(
                "Error output: {:?}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(())
    }
}

// Пример использования
impl Action {
    pub async fn execute(&self, event: &Event) -> Result<(), Error> {
        trace!("start check event");
        // Проверка, соответствует ли событие триггеру
        if self
            .triggers
            .iter()
            .any(|ek| matcher::match_event_kind(ek, &event.kind))
        {
            trace!("tracked event has been found {:#?}", event);
            // TODO: подумать над расспаралеливанием задач, мб на уровне акторов
            for path in event.paths.iter() {
                trace!("check path {:#?}", path);
                let metadata = tokio::fs::metadata(&path).await?;
                // Проверка всех условий
                let mut file = fs::File::open(path).await?;
                let mut buffer = vec![0; 512];
                let _ = file.read(&mut buffer).await?;
                trace!("read buf len: {:#?}", buffer.len());
                let inf = infer::get(&buffer);
                trace!("inf: {:#?}", inf);
                let matcher_type: Option<infer::MatcherType> = inf.map(|i| i.matcher_type());
                trace!("matcher_type: {:#?}", matcher_type);
                let args = CheckArgs {
                    file_metadata: metadata,
                    file_type: matcher_type,
                    file_path: path.to_owned(),
                };
                if self.conditions.check(&args) {
                    // if self.conditions.iter().all(|cond| cond.check(&args)) {
                    match &self.action_type {
                        ActionType::MoveFile(move_file_action) => {
                            move_file(path, &move_file_action.destination).await?;
                        }
                        ActionType::DeleteFile(delete_file_action) => {
                            trace!("Deleting file with force: {}", delete_file_action.force);
                            remove_file(path).await?;
                        }
                        ActionType::CreateSymlink(create_symlink_action) => {
                            create_symlink(path, &create_symlink_action.to).await?;
                        }
                        ActionType::Custom(custom_action) => {
                            trace!("run custom command {} ", &custom_action.command);
                            custom_action.execute_command(path).await?;
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

async fn move_file(src: &Path, dst: &Path) -> Result<(), Error> {
    //TODO: возможно стоит сделать проверку на то что файл с таким именем существует, и мб
    //переименовать его как то
    let file_name = match src.file_name() {
        Some(file_name) => file_name,
        None => {
            let err = "fail to move file, invalid filename";
            error!(err);
            // TODO: не ок
            return Ok(());
        }
    };
    trace!("dest before mut {:?}", dst);
    let dest = dst.join(file_name);
    trace!("Moving file to {:?}", dest);
    fs::rename(src, dest).await
}

async fn remove_file(path: &Path) -> Result<(), Error> {
    fs::remove_file(path).await
}

async fn create_symlink(src: &Path, dst: &Path) -> Result<(), Error> {
    let file_name = match src.file_name() {
        Some(file_name) => file_name,
        None => {
            let err = "fail to create simlink, invalid filename";
            error!(err);
            // TODO: не ок
            return Ok(());
        }
    };
    let to = src.join(file_name);
    trace!("Creating symlink from {:?} to {:?}", src, to);
    fs::symlink(src, dst).await
}
