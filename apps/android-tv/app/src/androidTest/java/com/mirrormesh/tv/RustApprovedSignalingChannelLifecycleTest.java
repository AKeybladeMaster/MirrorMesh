package com.mirrormesh.tv;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertFalse;
import static org.junit.Assert.assertTrue;

import androidx.test.ext.junit.runners.AndroidJUnit4;
import java.util.concurrent.BlockingQueue;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.LinkedBlockingQueue;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.concurrent.atomic.AtomicReference;
import org.junit.Test;
import org.junit.runner.RunWith;

/** Checks terminal Rust-approved signaling events release the TV control channel. */
@RunWith(AndroidJUnit4.class)
public final class RustApprovedSignalingChannelLifecycleTest {
    private static final long CONNECTION_ID = 53L;
    private static final String PEER_FINGERPRINT = "a".repeat(64);
    private static final long EVENT_TIMEOUT_SECONDS = 5;

    @Test
    public void remoteSessionStopNotifiesListenerAndClosesControlChannelOnce() throws Exception {
        LifecycleBridge bridge = new LifecycleBridge();
        RustApprovedSignalingChannel channel = new RustApprovedSignalingChannel(
                CONNECTION_ID, PEER_FINGERPRINT, bridge);
        CountDownLatch stopped = new CountDownLatch(1);
        channel.setRemoteStopListener(stopped::countDown);

        try {
            channel.start(ignored -> {});
            bridge.enqueue("{\"ok\":true,\"pending\":false,\"kind\":\"stop\"}");

            assertTrue("remote stop was not delivered", stopped.await(
                    EVENT_TIMEOUT_SECONDS, TimeUnit.SECONDS));
            assertTrue("remote stop did not close the native control channel",
                    bridge.awaitClosed());
            assertEquals("remote stop closed the channel more than once", 1, bridge.closeCalls());
            assertFalse("closed signaling channel still reports approved",
                    channel.isAuthenticatedAndApproved());
        } finally {
            channel.close();
        }
    }

    @Test
    public void pairingRevocationNotifiesListenerAndClosesControlChannelOnce() throws Exception {
        LifecycleBridge bridge = new LifecycleBridge();
        RustApprovedSignalingChannel channel = new RustApprovedSignalingChannel(
                CONNECTION_ID, PEER_FINGERPRINT, bridge);
        CountDownLatch revoked = new CountDownLatch(1);
        channel.setRevokedListener(revoked::countDown);

        try {
            channel.start(ignored -> {});
            bridge.enqueue("{\"ok\":true,\"pending\":false,\"kind\":\"revoke\"}");

            assertTrue("pairing revocation was not delivered", revoked.await(
                    EVENT_TIMEOUT_SECONDS, TimeUnit.SECONDS));
            assertTrue("revocation did not close the native control channel",
                    bridge.awaitClosed());
            assertEquals("revocation closed the channel more than once", 1, bridge.closeCalls());
            assertFalse("revoked signaling channel still reports approved",
                    channel.isAuthenticatedAndApproved());
        } finally {
            channel.close();
        }
    }

    @Test
    public void malformedControlMessageFailsClosedAndReportsFailure() throws Exception {
        LifecycleBridge bridge = new LifecycleBridge();
        RustApprovedSignalingChannel channel = new RustApprovedSignalingChannel(
                CONNECTION_ID, PEER_FINGERPRINT, bridge);
        CountDownLatch failed = new CountDownLatch(1);
        AtomicReference<String> failureCode = new AtomicReference<>();
        channel.setFailureListener(code -> {
            failureCode.set(code);
            failed.countDown();
        });

        try {
            channel.start(ignored -> {});
            bridge.enqueue("{not-json");

            assertTrue("malformed control message did not fail the channel",
                    failed.await(EVENT_TIMEOUT_SECONDS, TimeUnit.SECONDS));
            assertTrue("failed channel was not closed", bridge.awaitClosed());
            assertEquals("unexpected failure code", "signaling_channel_failed", failureCode.get());
            assertEquals("failed channel closed more than once", 1, bridge.closeCalls());
            assertFalse("failed signaling channel still reports approved",
                    channel.isAuthenticatedAndApproved());
        } finally {
            channel.close();
        }
    }

    private static final class LifecycleBridge
            implements RustApprovedSignalingChannel.NativeApprovedControlBridge {
        private final BlockingQueue<String> messages = new LinkedBlockingQueue<>();
        private final CountDownLatch closed = new CountDownLatch(1);
        private final AtomicInteger closeCalls = new AtomicInteger();
        private volatile boolean isClosed;

        void enqueue(String message) {
            messages.add(message);
        }

        boolean awaitClosed() throws InterruptedException {
            return closed.await(EVENT_TIMEOUT_SECONDS, TimeUnit.SECONDS);
        }

        int closeCalls() {
            return closeCalls.get();
        }

        @Override
        public String pollControlMessage(long connectionId) throws InterruptedException {
            String message = messages.poll(100, TimeUnit.MILLISECONDS);
            return message == null
                    ? "{\"ok\":true,\"pending\":true}"
                    : message;
        }

        @Override
        public String sendSessionAnswer(long connectionId, String sdp) {
            return "{\"ok\":true}";
        }

        @Override
        public String sendSessionIce(
                long connectionId,
                String mediaId,
                int mediaLineIndex,
                String candidate) {
            return "{\"ok\":true}";
        }

        @Override
        public synchronized boolean closeControlChannel(long connectionId) {
            if (!isClosed) {
                isClosed = true;
                closeCalls.incrementAndGet();
                closed.countDown();
            }
            return true;
        }
    }
}
