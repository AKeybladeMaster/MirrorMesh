package com.mirrormesh.tv;

import java.util.concurrent.Executors;
import java.util.concurrent.ScheduledExecutorService;
import java.util.concurrent.ScheduledFuture;
import java.util.concurrent.TimeUnit;

/** One-shot bounded grace period for an interrupted TV WebRTC connection. */
final class TvReconnectWatchdog implements AutoCloseable {
    static final long DEFAULT_TIMEOUT_MILLIS = 10_000;

    interface Cancellable {
        void cancel();
    }

    interface Scheduler extends AutoCloseable {
        Cancellable schedule(Runnable action, long delayMillis);

        @Override
        void close();
    }

    private final long timeoutMillis;
    private final Scheduler scheduler;
    private Cancellable pending;
    private long generation;
    private boolean closed;

    TvReconnectWatchdog(long timeoutMillis) {
        this(timeoutMillis, new ExecutorScheduler());
    }

    TvReconnectWatchdog(long timeoutMillis, Scheduler scheduler) {
        if (timeoutMillis <= 0 || scheduler == null) {
            throw new IllegalArgumentException("A positive timeout and scheduler are required.");
        }
        this.timeoutMillis = timeoutMillis;
        this.scheduler = scheduler;
    }

    boolean arm(Runnable onTimeout) {
        if (onTimeout == null) {
            throw new IllegalArgumentException("The timeout callback is required.");
        }
        final long scheduledGeneration;
        synchronized (this) {
            if (closed || pending != null) {
                return false;
            }
            scheduledGeneration = ++generation;
        }

        Cancellable scheduled = scheduler.schedule(
                () -> fire(scheduledGeneration, onTimeout), timeoutMillis);
        synchronized (this) {
            if (closed || generation != scheduledGeneration) {
                scheduled.cancel();
                return false;
            }
            pending = scheduled;
            return true;
        }
    }

    void cancel() {
        Cancellable scheduled;
        synchronized (this) {
            generation++;
            scheduled = pending;
            pending = null;
        }
        if (scheduled != null) {
            scheduled.cancel();
        }
    }

    private void fire(long scheduledGeneration, Runnable onTimeout) {
        synchronized (this) {
            if (closed || generation != scheduledGeneration || pending == null) {
                return;
            }
            pending = null;
            generation++;
        }
        onTimeout.run();
    }

    @Override
    public void close() {
        Cancellable scheduled;
        synchronized (this) {
            if (closed) {
                return;
            }
            closed = true;
            generation++;
            scheduled = pending;
            pending = null;
        }
        if (scheduled != null) {
            scheduled.cancel();
        }
        scheduler.close();
    }

    private static final class ExecutorScheduler implements Scheduler {
        private final ScheduledExecutorService executor = Executors.newSingleThreadScheduledExecutor(
                action -> {
                    Thread thread = new Thread(action, "mirrormesh-tv-reconnect-watchdog");
                    thread.setDaemon(true);
                    return thread;
                });

        @Override
        public Cancellable schedule(Runnable action, long delayMillis) {
            ScheduledFuture<?> future = executor.schedule(
                    action, delayMillis, TimeUnit.MILLISECONDS);
            return () -> future.cancel(false);
        }

        @Override
        public void close() {
            executor.shutdown();
        }
    }
}
