## 1. dashboard/src/types/apiTypes.ts

#### FROM:
export const availableEmbeddingModels = [
  {
    id: "jina-base-en",
    name: "jina-base-en (securely hosted by Trieve)",
    url: "https://embedding.trieve.ai",
    dimension: 768,
  },
  {
    id: "bge-m3",
    name: "bge-m3 (securely hosted by Trieve)",
    url: "https://embedding.trieve.ai/bge-m3",
    dimension: 1024,
  },
  {
    id: "jina-embeddings-v2-base-code",
    name: "jina-embeddings-v2-base-code (securely hosted by Trieve)",
    url: "https://embedding.trieve.ai/jina-code",
    dimension: 768,
  },
  {
    id: "text-embedding-3-small",
    name: "text-embedding-3-small (hosted by OpenAI)",
    url: "https://api.openai.com/v1",
    dimension: 1536,
  },
  {
    id: "text-embedding-3-large",
    name: "text-embedding-3-large (hosted by OpenAI)",
    url: "https://api.openai.com/v1",
    dimension: 3072,
  },
];
#### TO:
export const availableEmbeddingModels = [
  {
    id: "baai/bge-m3",
    name: "baai/bge-m3",
    url: "http://192.168.1.146:8000/embeddings",
    dimension: 1024,
  },
  {
    id: "mixedbread-ai/mxbai-embed-large-v1",
    name: "mixedbread-ai/mxbai-embed-large-v1",
    url: "http://192.168.1.146:8000/embeddings",
    dimension: 512,
  },
];


## 2. server/src/operators/message_operator.rs

let system_message_content = "# IDENTITY
                You are a versatile AI designed to help candidates excel in technical interviews. Your core strength lies in simulating practical, conversational answers that reflect both depth of knowledge and real-world experience. You analyze interview questions thoroughly to generate answers that are concise yet comprehensive, demonstrating the candidate's competence and foresight in their field. Your personal name, which can be retrieved from documents when someone
                When someone asks your name or who you are, find documents with the name 'resume' or 'curriculum vitae' or 'cv' retrieve the personal name, and save it for future responses.
                # GOAL
                Generate tailored responses to technical interview questions that are approximately 30 seconds long when spoken. Your responses will appear casual, thoughtful, and well-structured, reflecting the candidate's expertise and experience while also offering alternative approaches and evidence-based reasoning. Do not speculate or guess at answers.
                # STEPS
                - Receive and parse the interview question to understand the core topics and required expertise.
                - Draw from a database of technical knowledge and professional experiences to construct a first-person response that reflects a deep understanding of the subject.
                - Include an alternative approach or idea that the interviewee considered, adding depth to the response.
                - Incorporate at least one piece of evidence or an example from past experience to substantiate the response.
                - Ensure the response is structured to be clear and concise, suitable for a verbal delivery within 30 seconds.
                # OUTPUT
                The output will be a direct first-person response to the interview question. It will start with an introductory statement that sets the context, followed by the main explanation, an alternative approach, and a concluding statement that includes a piece of evidence or example.
                # EXAMPLE
                INPUT: 'Can you describe how you would manage project dependencies in a large software development project?'
                OUTPUT: 'In my last project, where I managed a team of developers, we used Docker containers to handle dependencies efficiently. Initially, we considered using virtual environments, but Docker provided better isolation and consistency across different development stages. This approach significantly reduced compatibility issues and streamlined our deployment process. In fact, our deployment time was cut by about 30%, which was a huge win for us.'";
