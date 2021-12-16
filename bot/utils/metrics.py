import time

from prometheus_client import Gauge, Histogram


class Timer:
    def __init__(self):
        self.start = time.time()

    def stop(self) -> float:
        return time.time() - self.start


gateway_gauge = Gauge(
    name="discord_gateway_response_time_seconds",
    documentation="Delay between Heartbeats and AKGs",
)
api_gauge = Gauge(
    name="discord_api_response_time_seconds",
    documentation="Delay between sending an API request and it being processed",
    labelnames=["action"],
)
api_gauge.labels(action="send")
api_gauge.labels(action="edit")
api_gauge.labels(action="delete")

api_processing_gauge = Gauge(
    name="discord_api_processing_time_seconds",
    documentation="The API processing time discord reports",
)

BUCKETS = (
    0.005,
    0.01,
    0.025,
    0.05,
    0.075,
    0.1,
    0.25,
    0.5,
    0.75,
    1.0,
    2.5,
    5.0,
    7.5,
    10.0,
    float("INF"),
)

api_histogram = Histogram(
    name="discord_api_request_duration_seconds",
    documentation="Response time for requests made to the discord API",
    labelnames=["method", "path"],
    buckets=BUCKETS,
)
command_histogram = Histogram(
    name="discord_command_processing_time_seconds",
    documentation="Processing time for commands",
    labelnames=["command"],
    buckets=BUCKETS,
)
third_party_api_histogram = Histogram(
    name="discord_3rd_party_api_request_duration_seconds",
    documentation="Response time for the various APIs used by the bot",
    labelnames=["method", "url"],
    buckets=BUCKETS,
)
