from sentence_transformers import SentenceTransformer
from fastapi import FastAPI
import numpy as np
from typing import Union

model = SentenceTransformer('all-MiniLM-L6-v2')
app = FastAPI()

@app.get("/embed")
def embed(q: Union[str, None] = None):
  return {"embedding": model.encode(q).tolist()}

@app.get("/pulse")
def pulse():
  return {}
