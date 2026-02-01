#!/bin/bash
# This script acts as a wrapper for the ffmpeg command.
# It unsets the LD_LIBRARY_PATH environment variable before executing ffmpeg.
# This is necessary to prevent conflicts with libraries from other environments,
# such as Anaconda, which was causing ffmpeg to fail because it couldn't find
# the correct version of GLIBCXX.

unset LD_LIBRARY_PATH
/usr/bin/ffmpeg "$@"
