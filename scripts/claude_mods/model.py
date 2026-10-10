import gzip
import json
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer


def assistant(text):
    return {
        "type": "response.output_item.done",
        "item": {
            "type": "message",
            "role": "assistant",
            "id": "message",
            "content": [{"type": "output_text", "text": text}],
        },
    }


def tool(name, arguments, identity="native-call"):
    return {
        "type": "response.output_item.done",
        "item": {
            "type": "function_call",
            "name": name,
            "call_id": identity,
            "arguments": json.dumps(arguments),
        },
    }


def patch(text, identity):
    return {
        "type": "response.output_item.done",
        "item": {
            "type": "custom_tool_call",
            "name": "apply_patch",
            "call_id": identity,
            "input": text,
        },
    }


class ModelServer:
    def __init__(self):
        self.requests = []
        self.next_items = []
        self.lock = threading.Lock()
        owner = self

        class Handler(BaseHTTPRequestHandler):
            def log_message(self, *_arguments):
                return

            def do_GET(self):
                self.send_response(404)
                self.end_headers()

            def do_POST(self):
                data = self.rfile.read(int(self.headers["Content-Length"]))
                if self.headers.get("Content-Encoding") == "gzip":
                    data = gzip.decompress(data)
                request = json.loads(data)
                with owner.lock:
                    owner.requests.append(request)
                    helper = request.get("max_output_tokens") is not None
                    if helper:
                        content = owner.helper(request)
                        items = [assistant(content)]
                    elif owner.next_items:
                        items = owner.next_items.pop(0)
                    else:
                        items = [assistant("The requested change is complete.")]
                    identity = f"response-{len(owner.requests)}"
                events = [{"type": "response.created", "response": {"id": identity}}]
                if not helper:
                    for event in items:
                        events.append(
                            {
                                "type": "response.output_item.added",
                                "item": event["item"],
                            }
                        )
                        if event["item"]["type"] == "message":
                            for part in event["item"]["content"]:
                                events.append(
                                    {
                                        "type": "response.output_text.delta",
                                        "delta": part["text"],
                                    }
                                )
                events.extend(items)
                events.append(
                    {
                        "type": "response.completed",
                        "response": {
                            "id": identity,
                            "usage": {
                                "input_tokens": 20,
                                "output_tokens": 10,
                                "total_tokens": 30,
                            },
                        },
                    }
                )
                body = "".join(
                    f"data: {json.dumps(event)}\n\n" for event in events
                ).encode()
                self.send_response(200)
                self.send_header("Content-Type", "text/event-stream")
                self.send_header("Content-Length", str(len(body)))
                self.end_headers()
                self.wfile.write(body)

        self.server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        threading.Thread(target=self.server.serve_forever, daemon=True).start()
        self.url = f"http://127.0.0.1:{self.server.server_port}/v1"

    def helper(self, request):
        text = json.dumps(request.get("input", [])) + request.get("instructions", "")
        if "verdict" in text:
            return json.dumps(
                {
                    "verdict": "clear"
                    if "unrelated/" in json.dumps(request.get("input", []))
                    else "in-scope",
                    "reason": "Check the requested file scope.",
                }
            )
        if "decisions" in text:
            return json.dumps(
                {
                    "decisions": [
                        {
                            "title": "Use a native patch",
                            "choice": "The agent changes the requested file.",
                            "why": "The user requested the change.",
                            "evidence": "The patch updates value.txt.",
                            "keep": "Keep edits within the request.",
                            "avoid": "Avoid unrelated files.",
                        }
                    ]
                }
            )
        if "questions" in text:
            return json.dumps(
                {
                    "questions": [
                        {
                            "text": "Which result is required?",
                            "why": "This sets the scope.",
                            "options": ["One", "Two"],
                        }
                    ]
                }
            )
        return json.dumps(
            {
                "title": "Update the requested value",
                "explanation": "The patch changes the requested value. The native tool writes the file.",
                "before": [{"text": "The file contains one.", "editIds": [1]}],
                "after": [
                    {"text": "The file contains two.", "changed": True, "editIds": [1]}
                ],
                "edits": [{"id": 1, "why": "The user requested two."}],
            }
        )

    def close(self):
        self.server.shutdown()
        self.server.server_close()
