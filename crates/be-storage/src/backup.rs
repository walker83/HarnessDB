use std::path::{Path, PathBuf};

use crate::rowset::Rowset;
use crate::segment::{SegmentReader, SegmentWriter};
use crate::tablet::{Tablet, TabletSchema as TabletSchemaType};

#[derive(Debug, Clone)]
pub struct TabletExporter;

impl TabletExporter {
    pub async fn export_tablet(
        tablet: &Tablet,
        export_path: &Path,
    ) -> Result<TabletExportMeta, String> {
        tokio::fs::create_dir_all(export_path)
            .await
            .map_err(|e| format!("Failed to create export dir: {}", e))?;

        let block = tablet.read(None, &[])?;

        let tablet_meta = TabletExportMeta {
            tablet_id: tablet.tablet_id,
            schema: tablet.schema.clone(),
            num_rows: block.num_rows() as u64,
        };

        let schema_path = export_path.join("tablet_meta.json");
        let schema_json = serde_json::to_vec(&tablet_meta)
            .map_err(|e| format!("Serialize error: {}", e))?;
        tokio::fs::write(&schema_path, schema_json).await
            .map_err(|e| format!("Write error: {}", e))?;

        let data_path = export_path.join("tablet_data.rov");
        let file_size = SegmentWriter::write_segment(&data_path, &block)
            .map_err(|e| format!("Write segment error: {}", e))?;

        let seg_meta_path = export_path.join("segment_meta.json");
        let seg_meta = SegmentExportMeta {
            segment_id: 0,
            path: data_path.to_string_lossy().to_string(),
            num_rows: block.num_rows() as u64,
            size: file_size,
        };
        let seg_json = serde_json::to_vec(&seg_meta)
            .map_err(|e| format!("Serialize error: {}", e))?;
        tokio::fs::write(&seg_meta_path, seg_json).await
            .map_err(|e| format!("Write error: {}", e))?;

        Ok(TabletExportMeta {
            tablet_id: tablet.tablet_id,
            schema: tablet.schema.clone(),
            num_rows: block.num_rows() as u64,
        })
    }

    pub async fn import_tablet(
        tablet_id: u64,
        schema: TabletSchemaType,
        import_path: &Path,
        data_dir: PathBuf,
    ) -> Result<Tablet, String> {
        let data_path = import_path.join("tablet_data.rov");
        if !data_path.exists() {
            return Err(format!("Data file not found: {:?}", data_path));
        }

        let tablet = Tablet::new(tablet_id, schema.clone(), data_dir);

        let block = SegmentReader::scan_segment(&data_path, None, &[])
            .map_err(|e| format!("Read segment error: {}", e))?;

        tablet.write(&block)
            .map_err(|e| format!("Write error: {}", e))?;

        Ok(tablet)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TabletExportMeta {
    pub tablet_id: u64,
    pub schema: TabletSchemaType,
    pub num_rows: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SegmentExportMeta {
    pub segment_id: u64,
    pub path: String,
    pub num_rows: u64,
    pub size: u64,
}

pub struct RowsetExporter;

impl RowsetExporter {
    pub async fn export_rowset(
        rowset: &Rowset,
        export_path: &Path,
    ) -> Result<RowsetExportMeta, String> {
        tokio::fs::create_dir_all(export_path)
            .await
            .map_err(|e| format!("Failed to create export dir: {}", e))?;

        let rowset_meta = RowsetExportMeta {
            rowset_id: rowset.meta.rowset_id,
            tablet_id: rowset.meta.tablet_id,
            version: rowset.meta.version,
            segments: rowset.segments.iter().map(|s| SegmentExportMeta {
                segment_id: s.segment_id,
                path: s.path.clone(),
                num_rows: s.num_rows,
                size: s.size,
            }).collect(),
        };

        let meta_path = export_path.join("rowset_meta.json");
        let meta_json = serde_json::to_vec(&rowset_meta)
            .map_err(|e| format!("Serialize error: {}", e))?;
        tokio::fs::write(&meta_path, meta_json).await
            .map_err(|e| format!("Write error: {}", e))?;

        for seg in &rowset.segments {
            let src = Path::new(&seg.path);
            let dst = export_path.join(format!("seg_{}.dat", seg.segment_id));
            tokio::fs::copy(src, &dst).await
                .map_err(|e| format!("Copy error: {}", e))?;
        }

        Ok(rowset_meta)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RowsetExportMeta {
    pub rowset_id: u64,
    pub tablet_id: u64,
    pub version: u64,
    pub segments: Vec<SegmentExportMeta>,
}
