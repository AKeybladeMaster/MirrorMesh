package com.mirrormesh.tv;

/** JDK-only deterministic checks for reconnect deadline scheduling and cleanup. */
public final class TvReconnectWatchdogTest {
    private TvReconnectWatchdogTest() {}

    public static void main(String[] args) {
        timeoutRunsOnce();
        recoveryCancelsTimeoutAndAllowsAnotherAttempt();
        closeCancelsTimeoutAndRejectsFutureAttempts();
        System.out.println("Android TV reconnect watchdog checks passed.");
    }

    private static void timeoutRunsOnce() {
        ManualScheduler scheduler = new ManualScheduler();
        TvReconnectWatchdog watchdog = new TvReconnectWatchdog(10_000, scheduler);
        int[] timeouts = {0};

        expect(true, watchdog.arm(() -> timeouts[0]++), "first reconnect deadline arms");
        expect(10_000L, scheduler.delayMillis, "reconnect deadline is bounded");
        expect(false, watchdog.arm(() -> timeouts[0]++), "duplicate deadline is ignored");
        scheduler.fire();
        scheduler.fire();

        expect(1, timeouts[0], "one timeout callback runs");
        watchdog.close();
    }

    private static void recoveryCancelsTimeoutAndAllowsAnotherAttempt() {
        ManualScheduler scheduler = new ManualScheduler();
        TvReconnectWatchdog watchdog = new TvReconnectWatchdog(10_000, scheduler);
        int[] timeouts = {0};

        watchdog.arm(() -> timeouts[0]++);
        watchdog.cancel();
        scheduler.fire();
        expect(0, timeouts[0], "recovered connection cancels its deadline");
        expect(true, watchdog.arm(() -> timeouts[0]++), "later interruption gets a deadline");
        scheduler.fire();
        expect(1, timeouts[0], "later interruption can expire");
        watchdog.close();
    }

    private static void closeCancelsTimeoutAndRejectsFutureAttempts() {
        ManualScheduler scheduler = new ManualScheduler();
        TvReconnectWatchdog watchdog = new TvReconnectWatchdog(10_000, scheduler);
        int[] timeouts = {0};

        watchdog.arm(() -> timeouts[0]++);
        watchdog.close();
        scheduler.fire();

        expect(0, timeouts[0], "closed receiver cancels its reconnect deadline");
        expect(true, scheduler.closed, "watchdog closes its scheduler");
        expect(false, watchdog.arm(() -> timeouts[0]++), "closed watchdog cannot be reused");
    }

    private static void expect(Object expected, Object actual, String label) {
        if (expected == null ? actual != null : !expected.equals(actual)) {
            throw new AssertionError(label + ": expected " + expected + ", got " + actual);
        }
    }

    private static final class ManualScheduler implements TvReconnectWatchdog.Scheduler {
        private Runnable action;
        private long delayMillis;
        private boolean cancelled;
        private boolean closed;

        @Override
        public TvReconnectWatchdog.Cancellable schedule(Runnable scheduled, long delay) {
            if (closed || action != null && !cancelled) {
                throw new IllegalStateException("A scheduled action is already pending.");
            }
            action = scheduled;
            delayMillis = delay;
            cancelled = false;
            return () -> cancelled = true;
        }

        private void fire() {
            Runnable scheduled = action;
            action = null;
            if (scheduled != null && !cancelled && !closed) {
                scheduled.run();
            }
        }

        @Override
        public void close() {
            closed = true;
        }
    }
}
