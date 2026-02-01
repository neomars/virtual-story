
const { pool: dbPool } = require('./db');
const fs = require('fs').promises;
const path = require('path');

const videosDir = path.join(__dirname, 'uploads/videos');
const thumbnailsDir = path.join(__dirname, 'uploads/thumbnails');

async function cleanupOrphanedFiles() {
  let connection;
  try {
    console.log('Starting cleanup process...');
    connection = await dbPool.getConnection();

    // 1. Get all video and thumbnail paths from the database
    const [scenes] = await connection.execute('SELECT video_path, thumbnail_path FROM scenes');
    const dbVideoFiles = new Set(scenes.map(s => path.basename(s.video_path)));
    const dbThumbnailFiles = new Set(scenes.map(s => path.basename(s.thumbnail_path)));

    console.log(`Found ${dbVideoFiles.size} video records and ${dbThumbnailFiles.size} thumbnail records in the database.`);

    // 2. Get all files from the uploads directories
    const [diskVideoFiles, diskThumbnailFiles] = await Promise.all([
      fs.readdir(videosDir),
      fs.readdir(thumbnailsDir)
    ]);

    console.log(`Found ${diskVideoFiles.length} files in the videos directory.`);
    console.log(`Found ${diskThumbnailFiles.length} files in the thumbnails directory.`);

    // 3. Find and delete orphaned video files
    const orphanedVideos = diskVideoFiles.filter(file => !dbVideoFiles.has(file));
    console.log(`Found ${orphanedVideos.length} orphaned video files to delete.`);
    for (const file of orphanedVideos) {
      const filePath = path.join(videosDir, file);
      try {
        await fs.unlink(filePath);
        console.log(`  - Deleted video: ${file}`);
      } catch (err) {
        console.error(`  - Error deleting video ${file}:`, err.message);
      }
    }

    // 4. Find and delete orphaned thumbnail files
    const orphanedThumbnails = diskThumbnailFiles.filter(file => !dbThumbnailFiles.has(file));
    console.log(`Found ${orphanedThumbnails.length} orphaned thumbnail files to delete.`);
    for (const file of orphanedThumbnails) {
      const filePath = path.join(thumbnailsDir, file);
      try {
        await fs.unlink(filePath);
        console.log(`  - Deleted thumbnail: ${file}`);
      } catch (err) {
        console.error(`  - Error deleting thumbnail ${file}:`, err.message);
      }
    }

    console.log('Cleanup process finished successfully.');

  } catch (error) {
    console.error('An error occurred during the cleanup process:', error);
  } finally {
    if (connection) {
      connection.release();
    }
    // We need to end the pool to allow the script to exit.
    // In a real application, you wouldn't do this if the server is running.
    dbPool.end();
  }
}

cleanupOrphanedFiles();
