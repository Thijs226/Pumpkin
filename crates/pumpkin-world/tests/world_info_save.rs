use std::fs;

use pumpkin_util::world_seed::Seed;
use pumpkin_world::world_info::{
    LevelData, WorldInfoWriter,
    anvil::{AnvilLevelInfo, LEVEL_DAT_BACKUP_FILE_NAME, LEVEL_DAT_FILE_NAME},
};
use tempfile::TempDir;

#[test]
fn write_world_info_preserves_level_dat_when_backup_fails() {
    let temp_dir = TempDir::new().unwrap();
    let mut data = LevelData::default(Seed(42));

    AnvilLevelInfo
        .write_world_info(&data, temp_dir.path())
        .unwrap();
    let original_level_dat = fs::read(temp_dir.path().join(LEVEL_DAT_FILE_NAME)).unwrap();

    fs::create_dir(temp_dir.path().join(LEVEL_DAT_BACKUP_FILE_NAME)).unwrap();
    data.level_name = "Updated World".to_string();

    let result = AnvilLevelInfo.write_world_info(&data, temp_dir.path());

    assert!(result.is_err(), "a failed level.dat backup must fail the save");
    assert_eq!(
        fs::read(temp_dir.path().join(LEVEL_DAT_FILE_NAME)).unwrap(),
        original_level_dat,
        "the last valid level.dat must stay untouched when its backup cannot be written"
    );
}
