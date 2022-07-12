# syntax=docker/dockerfile:1
FROM python:3.8
EXPOSE 8000
RUN pip install poetry
WORKDIR /code
COPY poetry.lock pyproject.toml /code/
RUN poetry config virtualenvs.create false \
  && poetry install --no-dev --no-interaction --no-ansi
RUN python -c 'from sentence_transformers import SentenceTransformer; SentenceTransformer("multi-qa-mpnet-base-dot-v1") '
COPY . /code
CMD uvicorn embedder.embedding_server:app --host 0.0.0.0 --workers 2
