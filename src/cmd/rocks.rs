use anyhow::Result;
use clap::Parser;
use indicatif::{ParallelProgressIterator, ProgressBar, ProgressIterator, ProgressStyle};
use log::info;
use rayon::prelude::*;
use std::sync::atomic::{AtomicU16, Ordering};

use crate::{config::Opts, db::init_db};

use super::SubCommandExtend;

#[derive(Parser, Debug, Clone)]
pub struct UpdateDB {}

impl SubCommandExtend for UpdateDB {
    #[tokio::main]
    async fn run(&self, opts: &Opts) -> Result<()> {
        let rocks = crate::rocks::ImageDB::open(&opts.conf_dir, true)?;
        if !opts.conf_dir.path().exists() {
            std::fs::create_dir_all(opts.conf_dir.path())?;
        }
        let db = init_db(opts.conf_dir.database(), true).await?;

        let pb_style = ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len} ({eta}) {msg}")
            .unwrap()
            .progress_chars("#>-");

        info!("正在统计特征点信息");
        let pb = ProgressBar::new(rocks.total_features()).with_style(pb_style.clone());
        let map = (0..rocks.total_images() as usize).map(|_| AtomicU16::new(0)).collect::<Vec<_>>();
        rocks.features().par_bridge().progress_with(pb).for_each(|features| {
            if let Ok((image_id, _feature_id)) = features {
                map[image_id as usize].fetch_add(1, Ordering::Relaxed);
            }
        });

        let mut tx = db.begin().await?;

        info!("正在迁移图片信息");
        let pb = ProgressBar::new(rocks.total_images()).with_style(pb_style.clone());
        for image in rocks.images().progress_with(pb.clone()) {
            let (id, hash, path) = image?;
            sqlx::query!("INSERT INTO image (id, hash, path) VALUES (?, ?, ?)", id, hash, path)
                .execute(&mut *tx)
                .await?;
        }

        info!("正在迁移特征点信息");
        let mut total_vector_count = 0;
        for i in (0..map.len()).progress_with(pb) {
            let vector_count = map[i as usize].load(Ordering::Relaxed) as i64;
            total_vector_count += vector_count;
            let i = i as i64;
            sqlx::query!(
                "INSERT INTO vector_stats (id, vector_count, total_vector_count, indexed) VALUES (?, ?, ?, 1)",
                i ,
                vector_count,
                total_vector_count
            )
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        Ok(())
    }
}
