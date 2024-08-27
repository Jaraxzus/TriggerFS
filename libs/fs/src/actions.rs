use crate::conditions::*;
use notify::event::{
    AccessKind, AccessMode, CreateKind, DataChange, MetadataKind, ModifyKind, RemoveKind,
    RenameMode,
};
use notify::{Event, EventKind, RecursiveMode};
use serde::Deserialize;
use std::io::Error;
use std::path::{Path, PathBuf};
use tokio::{fs, io::AsyncReadExt, process::Command};
use tracing::{error, trace};

#[derive(Debug, Deserialize, Clone)]
pub struct Action {
    triggers: Vec<EventKind>, // События файловой системы, на которые реагирует действие
    conditions: Vec<Condition>, // Условия для выполнения действия
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

// TODO: сомнительно, нужно персмотреть точно ли это будет так просто
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
            .any(|ek| match_event_kind(ek, &event.kind))
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
                if self.conditions.iter().all(|cond| cond.check(&args)) {
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

fn match_event_kind(conf_event: &EventKind, given_event: &EventKind) -> bool {
    match conf_event {
        EventKind::Any => true,
        EventKind::Access(access_kind) => match_access_kind(access_kind, given_event),
        EventKind::Create(create_kind) => match_create_kind(create_kind, given_event),
        EventKind::Modify(modify_kind) => match_modify_kind(modify_kind, given_event),
        EventKind::Remove(remove_kind) => match_remove_kind(remove_kind, given_event),
        EventKind::Other => conf_event == given_event,
    }
}

fn match_access_kind(conf_kind: &AccessKind, given_event: &EventKind) -> bool {
    if let EventKind::Access(given_kind) = given_event {
        match conf_kind {
            AccessKind::Any => true,
            AccessKind::Read => matches!(given_kind, AccessKind::Read),
            AccessKind::Open(mode) => match_access_mode(mode, given_kind),
            AccessKind::Close(mode) => match_access_mode(mode, given_kind),
            AccessKind::Other => conf_kind == given_kind,
        }
    } else {
        false
    }
}

fn match_access_mode(conf_mode: &AccessMode, given_kind: &AccessKind) -> bool {
    match given_kind {
        AccessKind::Open(mode) | AccessKind::Close(mode) => {
            *conf_mode == AccessMode::Any || mode == conf_mode
        }
        _ => false,
    }
}

fn match_create_kind(conf_kind: &CreateKind, given_event: &EventKind) -> bool {
    if let EventKind::Create(given_kind) = given_event {
        match conf_kind {
            CreateKind::Any => true,
            _ => conf_kind == given_kind,
        }
    } else {
        false
    }
}

fn match_modify_kind(conf_kind: &ModifyKind, given_event: &EventKind) -> bool {
    if let EventKind::Modify(given_kind) = given_event {
        match conf_kind {
            ModifyKind::Any => true,
            ModifyKind::Data(data_change) => match_data_change(data_change, given_kind),
            ModifyKind::Metadata(metadata_kind) => match_metadata_kind(metadata_kind, given_kind),
            ModifyKind::Name(rename_mode) => match_rename_mode(rename_mode, given_kind),
            ModifyKind::Other => conf_kind == given_kind,
        }
    } else {
        false
    }
}

fn match_data_change(conf_change: &DataChange, given_kind: &ModifyKind) -> bool {
    if let ModifyKind::Data(given_change) = given_kind {
        match conf_change {
            DataChange::Any => true,
            _ => conf_change == given_change,
        }
    } else {
        false
    }
}

fn match_metadata_kind(conf_kind: &MetadataKind, given_kind: &ModifyKind) -> bool {
    if let ModifyKind::Metadata(given_metadata) = given_kind {
        match conf_kind {
            MetadataKind::Any => true,
            _ => conf_kind == given_metadata,
        }
    } else {
        false
    }
}

fn match_rename_mode(conf_mode: &RenameMode, given_kind: &ModifyKind) -> bool {
    if let ModifyKind::Name(given_mode) = given_kind {
        match conf_mode {
            RenameMode::Any => true,
            _ => conf_mode == given_mode,
        }
    } else {
        false
    }
}

fn match_remove_kind(conf_kind: &RemoveKind, given_event: &EventKind) -> bool {
    if let EventKind::Remove(given_kind) = given_event {
        match conf_kind {
            RemoveKind::Any => true,
            _ => conf_kind == given_kind,
        }
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_match_event_kind_any() {
        // EventKind::Any should match all specific EventKind variants
        assert!(match_event_kind(
            &EventKind::Any,
            &EventKind::Access(AccessKind::Read)
        ));
        assert!(match_event_kind(
            &EventKind::Any,
            &EventKind::Access(AccessKind::Open(AccessMode::Read))
        ));
        assert!(match_event_kind(
            &EventKind::Any,
            &EventKind::Access(AccessKind::Close(AccessMode::Write))
        ));
        assert!(match_event_kind(
            &EventKind::Any,
            &EventKind::Access(AccessKind::Other)
        ));

        assert!(match_event_kind(
            &EventKind::Any,
            &EventKind::Create(CreateKind::File)
        ));
        assert!(match_event_kind(
            &EventKind::Any,
            &EventKind::Create(CreateKind::Folder)
        ));
        assert!(match_event_kind(
            &EventKind::Any,
            &EventKind::Create(CreateKind::Any)
        ));

        assert!(match_event_kind(
            &EventKind::Any,
            &EventKind::Modify(ModifyKind::Data(DataChange::Size))
        ));
        assert!(match_event_kind(
            &EventKind::Any,
            &EventKind::Modify(ModifyKind::Data(DataChange::Content))
        ));
        assert!(match_event_kind(
            &EventKind::Any,
            &EventKind::Modify(ModifyKind::Metadata(MetadataKind::WriteTime))
        ));
        assert!(match_event_kind(
            &EventKind::Any,
            &EventKind::Modify(ModifyKind::Metadata(MetadataKind::Permissions))
        ));
        assert!(match_event_kind(
            &EventKind::Any,
            &EventKind::Modify(ModifyKind::Name(RenameMode::To))
        ));
        assert!(match_event_kind(
            &EventKind::Any,
            &EventKind::Modify(ModifyKind::Name(RenameMode::From))
        ));
        assert!(match_event_kind(
            &EventKind::Any,
            &EventKind::Modify(ModifyKind::Any)
        ));
        assert!(match_event_kind(
            &EventKind::Any,
            &EventKind::Modify(ModifyKind::Other)
        ));

        assert!(match_event_kind(
            &EventKind::Any,
            &EventKind::Remove(RemoveKind::File)
        ));
        assert!(match_event_kind(
            &EventKind::Any,
            &EventKind::Remove(RemoveKind::Folder)
        ));
        assert!(match_event_kind(
            &EventKind::Any,
            &EventKind::Remove(RemoveKind::Any)
        ));

        assert!(match_event_kind(&EventKind::Any, &EventKind::Other));

        // EventKind::Any should match itself
        assert!(match_event_kind(&EventKind::Any, &EventKind::Any));

        // EventKind::Any should not match against a specific event that isn't EventKind::Any
        assert!(!match_event_kind(
            &EventKind::Access(AccessKind::Read),
            &EventKind::Create(CreateKind::File)
        ));
        assert!(!match_event_kind(
            &EventKind::Create(CreateKind::File),
            &EventKind::Modify(ModifyKind::Data(DataChange::Content))
        ));
        assert!(!match_event_kind(
            &EventKind::Modify(ModifyKind::Data(DataChange::Content)),
            &EventKind::Remove(RemoveKind::Folder)
        ));
        assert!(!match_event_kind(
            &EventKind::Remove(RemoveKind::File),
            &EventKind::Access(AccessKind::Open(AccessMode::Read))
        ));
    }

    #[test]
    fn test_match_event_kind_access() {
        // Define some AccessModes for easier reference
        let read_mode = AccessMode::Read;
        let write_mode = AccessMode::Write;
        let execute_mode = AccessMode::Execute;
        let any_mode = AccessMode::Any;

        // Test EventKind::Access with different AccessKind values
        assert!(match_event_kind(
            &EventKind::Access(AccessKind::Any),
            &EventKind::Access(AccessKind::Read)
        ));
        assert!(match_event_kind(
            &EventKind::Access(AccessKind::Any),
            &EventKind::Access(AccessKind::Open(read_mode))
        ));
        assert!(match_event_kind(
            &EventKind::Access(AccessKind::Any),
            &EventKind::Access(AccessKind::Close(write_mode))
        ));
        assert!(match_event_kind(
            &EventKind::Access(AccessKind::Any),
            &EventKind::Access(AccessKind::Other)
        ));

        // Test EventKind::Access with specific AccessKind values
        assert!(match_event_kind(
            &EventKind::Access(AccessKind::Read),
            &EventKind::Access(AccessKind::Read)
        ));
        assert!(!match_event_kind(
            &EventKind::Access(AccessKind::Read),
            &EventKind::Access(AccessKind::Open(read_mode))
        ));
        assert!(!match_event_kind(
            &EventKind::Access(AccessKind::Read),
            &EventKind::Access(AccessKind::Close(write_mode))
        ));
        assert!(!match_event_kind(
            &EventKind::Access(AccessKind::Read),
            &EventKind::Access(AccessKind::Other)
        ));

        // Test AccessKind::Open with different AccessMode values
        assert!(match_event_kind(
            &EventKind::Access(AccessKind::Open(read_mode)),
            &EventKind::Access(AccessKind::Open(read_mode))
        ));
        assert!(match_event_kind(
            &EventKind::Access(AccessKind::Open(any_mode)),
            &EventKind::Access(AccessKind::Open(read_mode))
        ));
        assert!(match_event_kind(
            &EventKind::Access(AccessKind::Open(any_mode)),
            &EventKind::Access(AccessKind::Open(execute_mode))
        ));
        assert!(!match_event_kind(
            &EventKind::Access(AccessKind::Open(read_mode)),
            &EventKind::Access(AccessKind::Open(write_mode))
        ));

        // Test AccessKind::Close with different AccessMode values
        assert!(match_event_kind(
            &EventKind::Access(AccessKind::Close(write_mode)),
            &EventKind::Access(AccessKind::Close(write_mode))
        ));
        assert!(match_event_kind(
            &EventKind::Access(AccessKind::Close(any_mode)),
            &EventKind::Access(AccessKind::Close(write_mode))
        ));
        assert!(match_event_kind(
            &EventKind::Access(AccessKind::Close(any_mode)),
            &EventKind::Access(AccessKind::Close(execute_mode))
        ));
        assert!(!match_event_kind(
            &EventKind::Access(AccessKind::Close(write_mode)),
            &EventKind::Access(AccessKind::Close(read_mode))
        ));

        // Test EventKind::Access with AccessKind::Other
        assert!(match_event_kind(
            &EventKind::Access(AccessKind::Other),
            &EventKind::Access(AccessKind::Other)
        ));
        assert!(!match_event_kind(
            &EventKind::Access(AccessKind::Other),
            &EventKind::Access(AccessKind::Read)
        ));
        assert!(!match_event_kind(
            &EventKind::Access(AccessKind::Other),
            &EventKind::Access(AccessKind::Open(read_mode))
        ));
        assert!(!match_event_kind(
            &EventKind::Access(AccessKind::Other),
            &EventKind::Access(AccessKind::Close(write_mode))
        ));

        // Test cases where EventKind is not Access
        assert!(!match_event_kind(
            &EventKind::Access(AccessKind::Read),
            &EventKind::Create(CreateKind::File)
        ));
        assert!(!match_event_kind(
            &EventKind::Access(AccessKind::Open(read_mode)),
            &EventKind::Modify(ModifyKind::Data(DataChange::Content))
        ));
        assert!(!match_event_kind(
            &EventKind::Access(AccessKind::Close(write_mode)),
            &EventKind::Remove(RemoveKind::File)
        ));
    }

    #[test]
    fn test_match_event_kind_create() {
        // Define some CreateKind values for easier reference
        let file_create = CreateKind::File;
        let folder_create = CreateKind::Folder;
        let any_create = CreateKind::Any;
        let other_create = CreateKind::Other;

        // Test EventKind::Create with different CreateKind values
        assert!(match_event_kind(
            &EventKind::Create(any_create),
            &EventKind::Create(CreateKind::File)
        ));
        assert!(match_event_kind(
            &EventKind::Create(any_create),
            &EventKind::Create(CreateKind::Folder)
        ));
        assert!(match_event_kind(
            &EventKind::Create(any_create),
            &EventKind::Create(CreateKind::Other)
        ));

        // Test EventKind::Create with specific CreateKind values
        assert!(match_event_kind(
            &EventKind::Create(file_create),
            &EventKind::Create(file_create)
        ));
        assert!(!match_event_kind(
            &EventKind::Create(file_create),
            &EventKind::Create(folder_create)
        ));
        assert!(!match_event_kind(
            &EventKind::Create(file_create),
            &EventKind::Create(other_create)
        ));

        assert!(match_event_kind(
            &EventKind::Create(folder_create),
            &EventKind::Create(folder_create)
        ));
        assert!(!match_event_kind(
            &EventKind::Create(folder_create),
            &EventKind::Create(file_create)
        ));
        assert!(!match_event_kind(
            &EventKind::Create(folder_create),
            &EventKind::Create(other_create)
        ));

        assert!(match_event_kind(
            &EventKind::Create(other_create),
            &EventKind::Create(other_create)
        ));
        assert!(!match_event_kind(
            &EventKind::Create(other_create),
            &EventKind::Create(file_create)
        ));
        assert!(!match_event_kind(
            &EventKind::Create(other_create),
            &EventKind::Create(folder_create)
        ));

        // Test cases where EventKind is not Create
        assert!(!match_event_kind(
            &EventKind::Create(file_create),
            &EventKind::Access(AccessKind::Read)
        ));
        assert!(!match_event_kind(
            &EventKind::Create(folder_create),
            &EventKind::Modify(ModifyKind::Data(DataChange::Size))
        ));
        assert!(!match_event_kind(
            &EventKind::Create(other_create),
            &EventKind::Remove(RemoveKind::Folder)
        ));
    }
    #[test]
    fn test_match_event_kind_modify() {
        // Define some ModifyKind values for easier reference
        let any_modify = ModifyKind::Any;
        let data_change_size = DataChange::Size;
        let metadata_access_time = MetadataKind::AccessTime;
        let rename_to = RenameMode::To;
        let other_modify = ModifyKind::Other;

        // Define some nested ModifyKind values
        let data_modify = ModifyKind::Data(data_change_size);
        let metadata_modify = ModifyKind::Metadata(metadata_access_time);
        let rename_modify = ModifyKind::Name(rename_to);

        // Test EventKind::Modify with ModifyKind::Any
        assert!(match_event_kind(
            &EventKind::Modify(any_modify),
            &EventKind::Modify(ModifyKind::Data(DataChange::Size))
        ));
        assert!(match_event_kind(
            &EventKind::Modify(any_modify),
            &EventKind::Modify(ModifyKind::Metadata(MetadataKind::AccessTime))
        ));
        assert!(match_event_kind(
            &EventKind::Modify(any_modify),
            &EventKind::Modify(ModifyKind::Name(RenameMode::To))
        ));
        assert!(match_event_kind(
            &EventKind::Modify(any_modify),
            &EventKind::Modify(ModifyKind::Other)
        ));

        // Test EventKind::Modify with specific ModifyKind::Data
        assert!(match_event_kind(
            &EventKind::Modify(data_modify),
            &EventKind::Modify(data_modify)
        ));
        assert!(!match_event_kind(
            &EventKind::Modify(data_modify),
            &EventKind::Modify(ModifyKind::Metadata(MetadataKind::WriteTime))
        ));
        assert!(!match_event_kind(
            &EventKind::Modify(data_modify),
            &EventKind::Modify(ModifyKind::Name(RenameMode::From))
        ));

        // Test EventKind::Modify with specific ModifyKind::Metadata
        assert!(match_event_kind(
            &EventKind::Modify(metadata_modify),
            &EventKind::Modify(metadata_modify)
        ));
        assert!(!match_event_kind(
            &EventKind::Modify(metadata_modify),
            &EventKind::Modify(ModifyKind::Data(DataChange::Content))
        ));
        assert!(!match_event_kind(
            &EventKind::Modify(metadata_modify),
            &EventKind::Modify(ModifyKind::Name(RenameMode::Both))
        ));

        // Test EventKind::Modify with specific ModifyKind::Name
        assert!(match_event_kind(
            &EventKind::Modify(rename_modify),
            &EventKind::Modify(rename_modify)
        ));
        assert!(!match_event_kind(
            &EventKind::Modify(rename_modify),
            &EventKind::Modify(ModifyKind::Data(DataChange::Size))
        ));
        assert!(!match_event_kind(
            &EventKind::Modify(rename_modify),
            &EventKind::Modify(ModifyKind::Metadata(MetadataKind::Permissions))
        ));

        // Test EventKind::Modify with ModifyKind::Other
        assert!(match_event_kind(
            &EventKind::Modify(other_modify),
            &EventKind::Modify(other_modify)
        ));
        assert!(!match_event_kind(
            &EventKind::Modify(other_modify),
            &EventKind::Modify(ModifyKind::Data(DataChange::Content))
        ));
        assert!(!match_event_kind(
            &EventKind::Modify(other_modify),
            &EventKind::Modify(ModifyKind::Metadata(MetadataKind::Ownership))
        ));
        assert!(!match_event_kind(
            &EventKind::Modify(other_modify),
            &EventKind::Modify(ModifyKind::Name(RenameMode::From))
        ));

        // Test cases where EventKind is not Modify
        assert!(!match_event_kind(
            &EventKind::Modify(data_modify),
            &EventKind::Access(AccessKind::Read)
        ));
        assert!(!match_event_kind(
            &EventKind::Modify(metadata_modify),
            &EventKind::Create(CreateKind::File)
        ));
        assert!(!match_event_kind(
            &EventKind::Modify(rename_modify),
            &EventKind::Remove(RemoveKind::Folder)
        ));
    }

    #[test]
    fn test_match_event_kind_remove() {
        // RemoveKind::Any should match any RemoveKind
        assert!(match_event_kind(
            &EventKind::Remove(RemoveKind::Any),
            &EventKind::Remove(RemoveKind::File)
        ));
        assert!(match_event_kind(
            &EventKind::Remove(RemoveKind::Any),
            &EventKind::Remove(RemoveKind::Folder)
        ));

        // Specific RemoveKind should match only the exact kind
        assert!(match_event_kind(
            &EventKind::Remove(RemoveKind::File),
            &EventKind::Remove(RemoveKind::File)
        ));
        assert!(!match_event_kind(
            &EventKind::Remove(RemoveKind::File),
            &EventKind::Remove(RemoveKind::Folder)
        ));
    }

    #[test]
    fn test_match_access_kind() {
        // AccessKind::Any should match any AccessKind
        assert!(match_access_kind(
            &AccessKind::Any,
            &EventKind::Access(AccessKind::Read)
        ));
        assert!(match_access_kind(
            &AccessKind::Any,
            &EventKind::Access(AccessKind::Open(AccessMode::Read))
        ));

        // Specific AccessKind should match only the exact kind
        assert!(match_access_kind(
            &AccessKind::Read,
            &EventKind::Access(AccessKind::Read)
        ));
        assert!(!match_access_kind(
            &AccessKind::Read,
            &EventKind::Access(AccessKind::Open(AccessMode::Read))
        ));

        // Other access kinds
        assert!(match_access_kind(
            &AccessKind::Open(AccessMode::Read),
            &EventKind::Access(AccessKind::Open(AccessMode::Read))
        ));
        assert!(!match_access_kind(
            &AccessKind::Open(AccessMode::Write),
            &EventKind::Access(AccessKind::Open(AccessMode::Read))
        ));
    }

    #[test]
    fn test_match_access_mode() {
        // AccessMode::Any should match any mode in Open or Close
        assert!(match_access_mode(
            &AccessMode::Any,
            &AccessKind::Open(AccessMode::Read)
        ));
        assert!(match_access_mode(
            &AccessMode::Any,
            &AccessKind::Close(AccessMode::Write)
        ));

        // Specific AccessMode should match only the exact mode
        assert!(match_access_mode(
            &AccessMode::Read,
            &AccessKind::Open(AccessMode::Read)
        ));
        assert!(!match_access_mode(
            &AccessMode::Write,
            &AccessKind::Open(AccessMode::Read)
        ));
    }

    #[test]
    fn test_match_create_kind() {
        // CreateKind::Any should match any CreateKind
        assert!(match_create_kind(
            &CreateKind::Any,
            &EventKind::Create(CreateKind::File)
        ));
        assert!(match_create_kind(
            &CreateKind::Any,
            &EventKind::Create(CreateKind::Folder)
        ));

        // Specific CreateKind should match only the exact kind
        assert!(match_create_kind(
            &CreateKind::File,
            &EventKind::Create(CreateKind::File)
        ));
        assert!(!match_create_kind(
            &CreateKind::File,
            &EventKind::Create(CreateKind::Folder)
        ));
    }

    #[test]
    fn test_match_modify_kind() {
        // ModifyKind::Any should match any ModifyKind
        assert!(match_modify_kind(
            &ModifyKind::Any,
            &EventKind::Modify(ModifyKind::Data(DataChange::Size))
        ));
        assert!(match_modify_kind(
            &ModifyKind::Any,
            &EventKind::Modify(ModifyKind::Metadata(MetadataKind::WriteTime))
        ));

        // Specific ModifyKind::Data should match only the exact data change kind
        assert!(match_modify_kind(
            &ModifyKind::Data(DataChange::Size),
            &EventKind::Modify(ModifyKind::Data(DataChange::Size))
        ));
        assert!(!match_modify_kind(
            &ModifyKind::Data(DataChange::Size),
            &EventKind::Modify(ModifyKind::Data(DataChange::Content))
        ));

        // Specific ModifyKind::Metadata should match only the exact metadata kind
        assert!(match_modify_kind(
            &ModifyKind::Metadata(MetadataKind::WriteTime),
            &EventKind::Modify(ModifyKind::Metadata(MetadataKind::WriteTime))
        ));
        assert!(!match_modify_kind(
            &ModifyKind::Metadata(MetadataKind::WriteTime),
            &EventKind::Modify(ModifyKind::Metadata(MetadataKind::Ownership))
        ));

        // Specific ModifyKind::Name should match only the exact rename mode
        assert!(match_modify_kind(
            &ModifyKind::Name(RenameMode::To),
            &EventKind::Modify(ModifyKind::Name(RenameMode::To))
        ));
        assert!(!match_modify_kind(
            &ModifyKind::Name(RenameMode::To),
            &EventKind::Modify(ModifyKind::Name(RenameMode::From))
        ));
    }

    #[test]
    fn test_match_data_change() {
        // DataChange::Any should match any data change
        assert!(match_data_change(
            &DataChange::Any,
            &ModifyKind::Data(DataChange::Content)
        ));
        assert!(match_data_change(
            &DataChange::Any,
            &ModifyKind::Data(DataChange::Size)
        ));

        // Specific DataChange should match only the exact kind
        assert!(match_data_change(
            &DataChange::Content,
            &ModifyKind::Data(DataChange::Content)
        ));
        assert!(!match_data_change(
            &DataChange::Size,
            &ModifyKind::Data(DataChange::Content)
        ));
    }

    #[test]
    fn test_match_metadata_kind() {
        // MetadataKind::Any should match any metadata kind
        assert!(match_metadata_kind(
            &MetadataKind::Any,
            &ModifyKind::Metadata(MetadataKind::WriteTime)
        ));
        assert!(match_metadata_kind(
            &MetadataKind::Any,
            &ModifyKind::Metadata(MetadataKind::Ownership)
        ));

        // Specific MetadataKind should match only the exact kind
        assert!(match_metadata_kind(
            &MetadataKind::WriteTime,
            &ModifyKind::Metadata(MetadataKind::WriteTime)
        ));
        assert!(!match_metadata_kind(
            &MetadataKind::Permissions,
            &ModifyKind::Metadata(MetadataKind::WriteTime)
        ));
    }

    #[test]
    fn test_match_rename_mode() {
        // RenameMode::Any should match any rename mode
        assert!(match_rename_mode(
            &RenameMode::Any,
            &ModifyKind::Name(RenameMode::To)
        ));
        assert!(match_rename_mode(
            &RenameMode::Any,
            &ModifyKind::Name(RenameMode::From)
        ));

        // Specific RenameMode should match only the exact mode
        assert!(match_rename_mode(
            &RenameMode::To,
            &ModifyKind::Name(RenameMode::To)
        ));
        assert!(!match_rename_mode(
            &RenameMode::To,
            &ModifyKind::Name(RenameMode::From)
        ));
    }

    #[test]
    fn test_match_remove_kind() {
        // RemoveKind::Any should match any remove kind
        assert!(match_remove_kind(
            &RemoveKind::Any,
            &EventKind::Remove(RemoveKind::Folder)
        ));
        assert!(match_remove_kind(
            &RemoveKind::Any,
            &EventKind::Remove(RemoveKind::File)
        ));

        // Specific RemoveKind should match only the exact kind
        assert!(match_remove_kind(
            &RemoveKind::File,
            &EventKind::Remove(RemoveKind::File)
        ));
        assert!(!match_remove_kind(
            &RemoveKind::File,
            &EventKind::Remove(RemoveKind::Folder)
        ));
    }
}
