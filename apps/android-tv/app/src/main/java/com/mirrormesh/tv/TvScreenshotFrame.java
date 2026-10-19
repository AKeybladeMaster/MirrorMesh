package com.mirrormesh.tv;

import java.nio.ByteBuffer;

/** Bounded, one-shot I420 to NV21 copy for a user-requested frame snapshot. */
public final class TvScreenshotFrame {
    public static final int MAX_WIDTH = 3840;
    public static final int MAX_HEIGHT = 2160;
    public static final int MAX_NV21_BYTES = MAX_WIDTH * MAX_HEIGHT * 3 / 2;

    private TvScreenshotFrame() {}

    public static byte[] toNv21(
            int width,
            int height,
            ByteBuffer yPlane,
            int yStride,
            ByteBuffer uPlane,
            int uStride,
            ByteBuffer vPlane,
            int vStride) {
        if (width <= 0 || height <= 0 || width > MAX_WIDTH || height > MAX_HEIGHT
                || (width & 1) != 0 || (height & 1) != 0) {
            throw new IllegalArgumentException("Unsupported screenshot dimensions.");
        }
        int chromaWidth = width / 2;
        int chromaHeight = height / 2;
        int outputLength = width * height * 3 / 2;
        if (outputLength > MAX_NV21_BYTES) {
            throw new IllegalArgumentException("Screenshot frame exceeds the size limit.");
        }

        int yBase = checkedBase(yPlane, yStride, height, width);
        int uBase = checkedBase(uPlane, uStride, chromaHeight, chromaWidth);
        int vBase = checkedBase(vPlane, vStride, chromaHeight, chromaWidth);
        ByteBuffer y = yPlane.duplicate();
        ByteBuffer u = uPlane.duplicate();
        ByteBuffer v = vPlane.duplicate();
        byte[] nv21 = new byte[outputLength];

        int outputOffset = 0;
        for (int row = 0; row < height; row++) {
            int inputOffset = yBase + row * yStride;
            for (int column = 0; column < width; column++) {
                nv21[outputOffset++] = y.get(inputOffset + column);
            }
        }
        for (int row = 0; row < chromaHeight; row++) {
            int uOffset = uBase + row * uStride;
            int vOffset = vBase + row * vStride;
            for (int column = 0; column < chromaWidth; column++) {
                nv21[outputOffset++] = v.get(vOffset + column);
                nv21[outputOffset++] = u.get(uOffset + column);
            }
        }
        return nv21;
    }

    private static int checkedBase(ByteBuffer plane, int stride, int rows, int columns) {
        if (plane == null || stride < columns || rows <= 0) {
            throw new IllegalArgumentException("Invalid screenshot plane.");
        }
        long required = (long) (rows - 1) * stride + columns;
        if (required > plane.remaining()) {
            throw new IllegalArgumentException("Screenshot plane is shorter than its stride.");
        }
        return plane.position();
    }
}
