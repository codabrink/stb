from sentence_transformers import SentenceTransformer
from fastapi import FastAPI
import numpy as np
from typing import Union

# model = SentenceTransformer('multi-qa-distilbert-cos-v1')
model = SentenceTransformer('multi-qa-mpnet-base-dot-v1')
app = FastAPI()

@app.get("/embed")
def embed(q: Union[str, None] = None):
  return {"embedding": model.encode(q).tolist()}

@app.get("/pulse")
def pulse():
  return {}
