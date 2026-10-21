# syntax=docker/dockerfile:1
FROM python:{{PY}}-slim-trixie

COPY --from=ghcr.io/astral-sh/uv:{{UV}} /uv /uvx /bin/

WORKDIR /app

RUN --mount=type=cache,target=/root/.cache/uv \
    --mount=type=bind,source=uv.lock,target=uv.lock \
    --mount=type=bind,source=pyproject.toml,target=pyproject.toml \
    uv sync --locked --no-install-project --no-dev

COPY . .

ENV PATH="/app/.venv/bin:$PATH"

RUN useradd --uid 65532 --no-create-home --shell /usr/sbin/nologin app
USER 65532

EXPOSE 8000

CMD ["uvicorn", "app.main:app", "--host", "0.0.0.0", "--port", "8000"]
