# Backup/Restore (P1)

## Status: DONE

## Implementation Summary

### Parser Support (crates/fe-sql-parser/src/parser.rs)
- CREATE REPOSITORY with S3/HDFS/Local types
- BACKUP DATABASE statement parsing
- RESTORE DATABASE statement parsing

### Backup Manager (crates/fe-common/src/backup.rs)
- BackupManager with repository and backup metadata management
- Repository struct supporting Local, S3, HDFS storage types
- Full backup metadata hierarchy: BackupMeta -> TableBackupMeta -> PartitionBackupMeta -> TabletBackupMeta -> RowsetBackupMeta
- Path builders for organized backup storage
- create_repository(), drop_repository(), list_repositories(), start_backup(), get_backup() operations
