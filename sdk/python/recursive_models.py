import re


def quote_recursive_root_models(source: str) -> str:
    pattern = re.compile(r"^(class (\w+)\(RootModel\[)(.*)(\]\):)$", re.MULTILINE)

    def replace(match: re.Match[str]) -> str:
        prefix, name, expression, suffix = match.groups()
        if expression.startswith(("'", '"')) or not re.search(
            rf"\b{re.escape(name)}\b", expression
        ):
            return match.group(0)
        return prefix + repr(expression) + suffix

    return pattern.sub(replace, source)
