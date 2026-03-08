#!/usr/bin/env bash
# 基于 refer/video/build_ffmpeg.sh 的 xun 版脚本（MSYS2 MinGW64）
# 目标：只覆盖 video compress/remux 所需能力，保留精简构建思路
#
# 用法：
#   bash tools/video/build_ffmpeg_msys2.sh
#
# 可选环境变量：
#   FFMPEG_PREFIX=$HOME/ffmpeg-xun
#   FFMPEG_SRC=$HOME/ffmpeg-src
#   FFMPEG_BRANCH=n8.0.1
#   ENABLE_HW=1
#   ENABLE_NONFREE=0
#   ENABLE_FDK_AAC=0
#   ENABLE_SHARED=1
#   ENABLE_SVTAV1=0

set -euo pipefail

FFMPEG_PREFIX="${FFMPEG_PREFIX:-$HOME/ffmpeg-xun}"
FFMPEG_SRC="${FFMPEG_SRC:-$HOME/ffmpeg-src}"
FFMPEG_BRANCH="${FFMPEG_BRANCH:-n8.0.1}"
ENABLE_HW="${ENABLE_HW:-1}"
ENABLE_NONFREE="${ENABLE_NONFREE:-0}"
ENABLE_FDK_AAC="${ENABLE_FDK_AAC:-0}"
ENABLE_SHARED="${ENABLE_SHARED:-1}"
ENABLE_SVTAV1="${ENABLE_SVTAV1:-0}"
JOBS="$(nproc)"

if [[ -z "${MSYSTEM:-}" || "${MSYSTEM}" != "MINGW64" ]]; then
  echo "ERROR: 请在 MSYS2 MinGW64 Shell 中运行。当前 MSYSTEM=${MSYSTEM:-<empty>}"
  exit 1
fi

PKGS=(
  mingw-w64-x86_64-toolchain
  mingw-w64-x86_64-nasm
  mingw-w64-x86_64-yasm
  mingw-w64-x86_64-pkgconf
  mingw-w64-x86_64-x264
  mingw-w64-x86_64-x265
  mingw-w64-x86_64-libvpx
  mingw-w64-x86_64-opus
  mingw-w64-x86_64-onevpl
  make
  git
  diffutils
)

if [[ "${ENABLE_FDK_AAC}" == "1" ]]; then
  PKGS+=(mingw-w64-x86_64-fdk-aac)
fi
if [[ "${ENABLE_SVTAV1}" == "1" ]]; then
  PKGS+=(mingw-w64-x86_64-svt-av1)
fi

echo "==> 安装依赖包..."
pacman -S --needed --noconfirm "${PKGS[@]}"

HW_FLAGS=()
if [[ "${ENABLE_HW}" == "1" ]]; then
  echo "==> 检查并安装 nv-codec-headers..."
  if [[ ! -d "$HOME/nv-codec-headers" ]]; then
    git clone --depth=1 https://github.com/FFmpeg/nv-codec-headers.git "$HOME/nv-codec-headers" || true
  fi
  if [[ -d "$HOME/nv-codec-headers" ]]; then
    (
      cd "$HOME/nv-codec-headers"
      make install PREFIX=/mingw64 || true
    )
  fi
  if [[ -f "/mingw64/include/ffnvcodec/nvEncodeAPI.h" ]]; then
    HW_FLAGS+=(--enable-nvenc --enable-nvdec --enable-cuvid)
    echo "==> NVENC/NVDEC/CUVID: enabled"
  else
    echo "==> NVENC/NVDEC/CUVID: not found, skip"
  fi
  if [[ -f "/mingw64/include/AMF/core/Factory.h" ]]; then
    HW_FLAGS+=(--enable-amf)
    echo "==> AMF: enabled"
  else
    echo "==> AMF: not found, skip"
  fi
  if pkg-config --exists vpl; then
    HW_FLAGS+=(--enable-libvpl)
    echo "==> oneVPL(QSV): enabled"
  else
    echo "==> oneVPL(QSV): not found, skip"
  fi
  HW_FLAGS+=(--enable-d3d11va --enable-d3d12va --enable-dxva2)
fi

if [[ ! -d "${FFMPEG_SRC}" ]]; then
  echo "==> 克隆 FFmpeg 源码..."
  git clone --depth=1 --branch "${FFMPEG_BRANCH}" https://github.com/FFmpeg/FFmpeg.git "${FFMPEG_SRC}"
else
  if [[ -d "${FFMPEG_SRC}/.git" ]]; then
    echo "==> 更新 FFmpeg 源码..."
    (
      cd "${FFMPEG_SRC}"
      if git rev-parse -q --verify "refs/tags/${FFMPEG_BRANCH}" >/dev/null || \
         git rev-parse -q --verify "refs/remotes/origin/${FFMPEG_BRANCH}" >/dev/null; then
        echo "==> 本地已存在目标引用 ${FFMPEG_BRANCH}，跳过 fetch"
      else
        git fetch --depth=1 --tags origin "${FFMPEG_BRANCH}"
      fi
      if git rev-parse -q --verify "refs/remotes/origin/${FFMPEG_BRANCH}" >/dev/null; then
        git checkout -B "${FFMPEG_BRANCH}" "origin/${FFMPEG_BRANCH}"
      elif git rev-parse -q --verify "refs/tags/${FFMPEG_BRANCH}" >/dev/null; then
        git checkout -f "${FFMPEG_BRANCH}"
      else
        git checkout -f FETCH_HEAD
      fi
    )
  else
    echo "==> 使用本地源码目录（非 git 仓库），跳过更新: ${FFMPEG_SRC}"
  fi
fi

cd "${FFMPEG_SRC}"

echo "==> 配置 FFmpeg..."
CFG=(
  --prefix="${FFMPEG_PREFIX}"
  --enable-gpl
  --enable-version3
  --enable-ffmpeg
  --enable-ffprobe
  --disable-ffplay
  --disable-doc
  --disable-debug
  --enable-w32threads
  --disable-everything
  --disable-autodetect

  --enable-decoder=h264
  --enable-decoder=hevc
  --enable-decoder=av1
  --enable-decoder=vp8
  --enable-decoder=vp9
  --enable-decoder=mpeg4
  --enable-decoder=aac
  --enable-decoder=mp3
  --enable-decoder=opus
  --enable-decoder=vorbis
  --enable-decoder=ac3
  --enable-decoder=eac3

  --enable-libx264
  --enable-encoder=libx264
  --enable-libx265
  --enable-encoder=libx265
  --enable-libvpx
  --enable-encoder=libvpx_vp9
  --enable-encoder=aac
  --enable-libopus
  --enable-encoder=libopus

  --enable-demuxer=mov
  --enable-demuxer=matroska
  --enable-demuxer=mpegts

  --enable-muxer=mp4
  --enable-muxer=matroska
  --enable-muxer=webm
  --enable-muxer=mpegts
  --enable-muxer=mov
  --enable-muxer=null

  --enable-protocol=file
  --enable-protocol=pipe

  --enable-filter=scale
  --enable-filter=hwupload
  --enable-filter=hwdownload
  --enable-filter=format
  --enable-filter=fps
  --enable-filter=crop
  --enable-filter=aformat

  --enable-optimizations
  --extra-cflags=-O3
  --extra-cflags=-march=native
  --extra-cflags=-fomit-frame-pointer
  --extra-ldflags=-Wl,--strip-all
  --target-os=mingw32
  --arch=x86_64
)

if [[ "${ENABLE_SHARED}" == "1" ]]; then
  CFG+=(--enable-shared --disable-static)
else
  CFG+=(--disable-shared --enable-static --pkg-config-flags=--static)
fi

if [[ "${ENABLE_NONFREE}" == "1" ]]; then
  CFG+=(--enable-nonfree)
else
  CFG+=(--disable-nonfree)
fi

if [[ "${ENABLE_FDK_AAC}" == "1" ]]; then
  CFG+=(--enable-libfdk_aac --enable-encoder=libfdk_aac)
fi
if [[ "${ENABLE_SVTAV1}" == "1" ]]; then
  CFG+=(--enable-libsvtav1 --enable-encoder=libsvtav1)
else
  # 默认关闭，避免在系统已安装 svt-av1 时被自动探测启用。
  CFG+=(--disable-libsvtav1)
fi

if [[ "${#HW_FLAGS[@]}" -gt 0 ]]; then
  CFG+=("${HW_FLAGS[@]}")
fi

./configure "${CFG[@]}"

echo "==> 编译 FFmpeg (jobs=${JOBS})..."
make -j"${JOBS}"

echo "==> 安装到 ${FFMPEG_PREFIX}..."
make install

echo ""
echo "==> 完成"
echo "FFmpeg: ${FFMPEG_PREFIX}/bin/ffmpeg.exe"
echo "FFprobe: ${FFMPEG_PREFIX}/bin/ffprobe.exe"
echo ""
echo "==> 建议验证"
echo "\"${FFMPEG_PREFIX}/bin/ffmpeg.exe\" -hide_banner -encoders | grep -E \"libx264|libx265|libsvtav1|libvpx-vp9|h264_nvenc|hevc_nvenc|h264_qsv|hevc_qsv|h264_amf|hevc_amf\""
echo "\"${FFMPEG_PREFIX}/bin/ffmpeg.exe\" -hide_banner -muxers | grep -E \"mp4|webm|matroska|mov\""
