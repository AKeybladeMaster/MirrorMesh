package com.mirrormesh.tv;

import java.nio.ByteBuffer;
import java.util.Arrays;

/** JDK-only checks for bounded I420 screenshot conversion. */
public final class TvScreenshotFrameTest {
    private TvScreenshotFrameTest() {}

    public static void main(String[] args) {
        copiesPaddedPlanesIntoNv21WithoutMutatingPositions();
        rejectsInvalidDimensionsAndShortPlanes();
        System.out.println("Android TV screenshot frame checks passed.");
    }

    private static void copiesPaddedPlanesIntoNv21WithoutMutatingPositions() {
        ByteBuffer y = ByteBuffer.wrap(new byte[] {
            99, 98, 1, 2, 3, 4, 97, 96, 5, 6, 7, 8, 95,
            94, 9, 10, 11, 12, 93, 92, 13, 14, 15, 16, 91, 90
        });
        ByteBuffer u = ByteBuffer.wrap(new byte[] {88, 21, 22, 87, 23, 24, 86, 85});
        ByteBuffer v = ByteBuffer.wrap(new byte[] {88, 89, 31, 32, 87, 86, 33, 34, 85});
        y.position(2);
        u.position(1);
        v.position(2);

        byte[] result = TvScreenshotFrame.toNv21(4, 4, y, 6, u, 3, v, 4);
        byte[] expected = new byte[] {
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
            31, 21, 32, 22, 33, 23, 34, 24
        };

        expect(Arrays.equals(expected, result), "I420 planes must become NV21 rows.");
        expect(y.position() == 2 && u.position() == 1 && v.position() == 2,
                "Input buffer positions must remain unchanged.");
    }

    private static void rejectsInvalidDimensionsAndShortPlanes() {
        expectThrows(() -> TvScreenshotFrame.toNv21(3, 2, plane(16), 3, plane(4), 2,
                plane(4), 2), "odd width");
        expectThrows(() -> TvScreenshotFrame.toNv21(3842, 2, plane(100), 3842,
                plane(100), 1921, plane(100), 1921), "oversized width");
        expectThrows(() -> TvScreenshotFrame.toNv21(4, 4, plane(8), 4, plane(4), 2,
                plane(4), 2), "short luma plane");
        expectThrows(() -> TvScreenshotFrame.toNv21(4, 4, plane(16), 4, plane(4), 1,
                plane(4), 2), "short chroma stride");
    }

    private static ByteBuffer plane(int size) {
        return ByteBuffer.allocate(size);
    }

    private static void expect(boolean condition, String message) {
        if (!condition) {
            throw new AssertionError(message);
        }
    }

    private static void expectThrows(Runnable action, String description) {
        try {
            action.run();
        } catch (IllegalArgumentException expected) {
            return;
        }
        throw new AssertionError("Expected invalid " + description + " to be rejected.");
    }
}
