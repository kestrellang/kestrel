#!/bin/bash
# Manages two kestrel-wall instances for zero-downtime restarts.
# Usage: wall.sh {start|stop|restart|status}

DIR="$(cd "$(dirname "$0")/.." && pwd)"
BIN="$DIR/kestrel-wall"
DB="$DIR/wall.db"
PORTS=(8080 8081)
MAX_RSS_KB=204800  # 200MB

start_instance() {
    local port=$1
    local pid_file="$DIR/wall-$port.pid"

    if [ -f "$pid_file" ] && kill -0 "$(cat "$pid_file")" 2>/dev/null; then
        echo "Instance on port $port already running (pid $(cat "$pid_file"))"
        return
    fi

    cd "$DIR"
    PORT=$port nohup "$BIN" > "$DIR/wall-$port.log" 2>&1 &
    echo $! > "$pid_file"
    echo "Started instance on port $port (pid $!)"
}

stop_instance() {
    local port=$1
    local pid_file="$DIR/wall-$port.pid"

    if [ -f "$pid_file" ]; then
        local pid=$(cat "$pid_file")
        if kill -0 "$pid" 2>/dev/null; then
            kill "$pid"
            echo "Stopped instance on port $port (pid $pid)"
        fi
        rm -f "$pid_file"
    else
        echo "No instance on port $port"
    fi
}

check_health() {
    local port=$1
    curl -sf -o /dev/null --connect-timeout 1 "http://127.0.0.1:$port/health"
}

check_rss() {
    local port=$1
    local pid_file="$DIR/wall-$port.pid"
    [ -f "$pid_file" ] || return 1
    local pid=$(cat "$pid_file")
    kill -0 "$pid" 2>/dev/null || return 1
    local rss=$(ps -o rss= -p "$pid" 2>/dev/null | tr -d ' ')
    [ -n "$rss" ] && [ "$rss" -gt "$MAX_RSS_KB" ]
}

case "${1:-}" in
    start)
        for port in "${PORTS[@]}"; do
            start_instance "$port"
        done
        ;;

    stop)
        for port in "${PORTS[@]}"; do
            stop_instance "$port"
        done
        ;;

    restart)
        # Rolling restart: one at a time so the other handles traffic
        for port in "${PORTS[@]}"; do
            echo "Restarting port $port..."
            stop_instance "$port"
            sleep 1
            start_instance "$port"
            # Wait for it to be healthy before restarting the other
            for i in $(seq 1 10); do
                if check_health "$port"; then
                    echo "Port $port healthy"
                    break
                fi
                sleep 0.5
            done
        done
        ;;

    status)
        for port in "${PORTS[@]}"; do
            local pid_file="$DIR/wall-$port.pid"
            if [ -f "$pid_file" ] && kill -0 "$(cat "$pid_file")" 2>/dev/null; then
                local pid=$(cat "$pid_file")
                local rss=$(ps -o rss= -p "$pid" 2>/dev/null | tr -d ' ')
                local health="unhealthy"
                check_health "$port" && health="healthy"
                echo "Port $port: running (pid $pid, RSS ${rss}KB, $health)"
            else
                echo "Port $port: stopped"
            fi
        done
        ;;

    watchdog)
        # Run from cron every 30s. Restarts unhealthy or memory-bloated instances.
        for port in "${PORTS[@]}"; do
            pid_file="$DIR/wall-$port.pid"

            # Check if process is alive
            if [ ! -f "$pid_file" ] || ! kill -0 "$(cat "$pid_file")" 2>/dev/null; then
                echo "$(date): Port $port dead, restarting"
                start_instance "$port"
                continue
            fi

            # Check health endpoint
            if ! check_health "$port"; then
                echo "$(date): Port $port unhealthy, restarting"
                stop_instance "$port"
                sleep 1
                start_instance "$port"
                continue
            fi

            # Check memory
            if check_rss "$port"; then
                echo "$(date): Port $port over ${MAX_RSS_KB}KB RSS, restarting"
                stop_instance "$port"
                sleep 1
                start_instance "$port"
            fi
        done
        ;;

    *)
        echo "Usage: $0 {start|stop|restart|status|watchdog}"
        exit 1
        ;;
esac
