import os
from dotenv import load_dotenv # Added import
from langchain_community.document_loaders import UnstructuredMarkdownLoader
from langchain.text_splitter import RecursiveCharacterTextSplitter
from langchain_google_genai import ChatGoogleGenerativeAI
from langchain_google_genai.embeddings import GoogleGenerativeAIEmbeddings
from langchain_community.vectorstores import Chroma
from langchain_core.prompts import ChatPromptTemplate
from langchain_core.output_parsers import StrOutputParser
from langchain_core.runnables import RunnablePassthrough

load_dotenv() # Load environment variables from .env

# Ensure GOOGLE_API_KEY is set as an environment variable
if not os.getenv("GOOGLE_API_KEY"):
    raise ValueError("GOOGLE_API_KEY environment variable not set.")

llm = ChatGoogleGenerativeAI(model="gemini-2.5-pro", temperature=0.7)
embeddings = GoogleGenerativeAIEmbeddings(model="models/embedding-001")

class ResearcherAgent:
    def __init__(self, data_path: str = "src/data", persist_directory: str = "src/chroma_db"):
        self.data_path = data_path
        self.persist_directory = persist_directory
        self.vectorstore = self._initialize_vectorstore()
        self.retriever = self.vectorstore.as_retriever()

        self.prompt = ChatPromptTemplate.from_messages([
            ("system", "You are an expert Marketing Researcher. Use the provided context to answer questions and provide relevant information for marketing campaigns. If the context does not contain the answer, state that you don't have enough information."),
            ("user", "Context: {context}\nQuestion: {question}")
        ])
        self.chain = {"context": self.retriever, "question": RunnablePassthrough()} | self.prompt | llm | StrOutputParser()

    def _initialize_vectorstore(self):
        """
        Loads and returns an existing ChromaDB vector store.
        Assumes the vector store has been pre-built by src/build_vectorstore.py.
        """
        if not os.path.exists(self.persist_directory) or not os.listdir(self.persist_directory):
            raise FileNotFoundError(
                f"Vector store not found at {self.persist_directory}. "
                "Please run `python3 src/build_vectorstore.py` to build the knowledge base."
            )
        print(f"Loading existing vector store from {self.persist_directory}...")
        vectorstore = Chroma(persist_directory=self.persist_directory, embedding_function=embeddings)
        return vectorstore

    def research(self, query: str) -> str:
        """
        Performs a RAG-based research query and returns the answer.
        """
        return self.chain.invoke(query)

if __name__ == "__main__":
    # Example usage (for testing)
    # Ensure src/data has some .md files and GOOGLE_API_KEY is set.
    # Note: The first run will create and populate the vector store.
    # Subsequent runs will load the existing one.
    researcher = ResearcherAgent()
    
    print("\n--- Researching Product Info ---")
    product_query = "What are the key benefits of the new organic cat food?"
    product_info = researcher.research(product_query)
    print(f"Query: {product_query}\nResult: {product_info}")

    print("\n--- Researching Brand Guidelines ---")
    brand_query = "What is the overall tone for Nature's Diet marketing?"
    brand_info = researcher.research(brand_query)
    print(f"Query: {brand_query}\nResult: {brand_info}")

    print("\n--- Researching unknown topic ---")
    unknown_query = "What is the capital of France?"
    unknown_info = researcher.research(unknown_query)
    print(f"Query: {unknown_query}\nResult: {unknown_info}")
