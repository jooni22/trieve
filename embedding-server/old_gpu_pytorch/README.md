# OLD GPU (NVIDIA) EMBEDDING AND RERANKING API SERVERS

The embedding and rerankig api server based on Sentence Transformers in Python with PyTorch support.
Only Nvidia GPU tested (TESLA M40 12GB) with low latency response ~50ms on localhost with small input data.
Probably should work with AMD aswell, but not tested. For more info read:
[https://pytorch.org/get-started/locally/]

## Basic execution

```bash
curl -X POST "http://localhost:8000/embeddings" \
     -H "Content-Type: application/json" \
     -d '{ "input": "test", "model": "baai/bge-m3" }'
```

```bash
curl -X POST "http://localhost:8000/embeddings" \
     -H "Content-Type: application/json" \
     -d '{ "input": "test", "model": "mixedbread-ai/mxbai-embed-large-v1" }'
```
