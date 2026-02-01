
const { exec } = require('child_process');
const path = require('path');
const fs = require('fs');

// This script attempts to generate a thumbnail using a direct ffmpeg command.
// It is designed to produce a clearer error message than the fluent-ffmpeg library.
//
// HOW TO RUN:
// node backend/test-ffmpeg.js /path/to/your/video.mp4

const videoFilePath = process.argv[2];

if (!videoFilePath) {
  console.error('ERROR: Please provide the full path to a video file as an argument.');
  console.error('Example: node backend/test-ffmpeg.js /home/mars/virtual-story/backend/uploads/videos/video-xxxxxxxx.mp4');
  process.exit(1);
}

if (!fs.existsSync(videoFilePath)) {
  console.error(`ERROR: The file does not exist at path: ${videoFilePath}`);
  process.exit(1);
}

const outputThumbnailPath = path.join('/tmp', `test-thumbnail-${Date.now()}.png`);
const ffmpegPath = '/usr/bin/ffmpeg';

// The raw ffmpeg command we are testing
const command = `${ffmpegPath} -i "${videoFilePath}" -ss 00:00:05.000 -vframes 1 -s 320x240 "${outputThumbnailPath}"`;

console.log('--- FFMPEG Direct Execution Test ---');
console.log(`Input video: ${videoFilePath}`);
console.log(`Output thumbnail: ${outputThumbnailPath}`);
console.log(`Executing command: ${command}\n`);

exec(command, (error, stdout, stderr) => {
  if (error) {
    console.error('--- TEST FAILED ---');
    console.error('An error occurred while executing the command.');
    console.error('\n[ERROR OBJECT]:');
    console.error(error);
    console.error('\n[STDERR]:');
    console.error(stderr);
    console.error('\n[STDOUT]:');
    console.error(stdout);
    console.log('\n--- END OF TEST ---');
    return;
  }

  console.log('--- TEST SUCCEEDED ---');
  console.log('The ffmpeg command executed successfully.');
  console.log(`Thumbnail should have been created at: ${outputThumbnailPath}`);
  console.log('\n[STDOUT]:');
  console.log(stdout);
  console.log('\n[STDERR]:');
  console.log(stderr); // ffmpeg often prints info to stderr even on success
  console.log('\n--- END OF TEST ---');
});
