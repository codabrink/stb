import json
import os
import numpy as np

from qdrant_client import QdrantClient

QDRANT_HOST = os.environ.get("QDRANT_HOST", "localhost")
QDRANT_PORT = os.environ.get("QDRANT_PORT", 6333)

BATCH_SIZE = 256

# qdrant_client = QdrantClient(host=)
