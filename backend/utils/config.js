const path = require('path');

const uploadsDir = path.join(__dirname, '../uploads');
const videosDir = path.join(uploadsDir, 'videos');
const thumbnailsDir = path.join(uploadsDir, 'thumbnails');
const partsDir = path.join(uploadsDir, 'parts');
const backgroundsDir = uploadsDir;

const videoFileFilter = (req, file, cb) => {
  const allowedExtensions = ['.mp4'];
  const ext = path.extname(file.originalname).toLowerCase();
  if (file.mimetype === 'video/mp4' && allowedExtensions.includes(ext)) {
    cb(null, true);
  } else {
    cb(new Error('Invalid file type. Only MP4 videos are allowed.'), false);
  }
};

module.exports = {
  uploadsDir,
  videosDir,
  thumbnailsDir,
  partsDir,
  backgroundsDir,
  videoFileFilter
};
