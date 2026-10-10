from openai_codex.generated.v2_all import ClaudeModRenderResponse


def test_recursive_mod_render_tree_preserves_controls_and_nested_children() -> None:
    payload = {
        "tree": {
            "element": "Box",
            "props": {"flexDirection": "column"},
            "children": [
                {
                    "element": "Button",
                    "props": {"label": "Keep", "onPress": {"callbackId": "current-control"}},
                    "children": [],
                },
                ["nested text", 2.0, None],
            ],
        }
    }
    response = ClaudeModRenderResponse.model_validate(payload)
    assert response.model_dump(mode="json", by_alias=True, exclude_unset=True) == payload
