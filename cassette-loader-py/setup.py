from setuptools import setup, find_packages

with open("README.md", "r", encoding="utf-8") as fh:
    long_description = fh.read()

setup(
    name="cassette-loader-py",
    version="1.0.0",
    author="Cassette Project",
    description="Python loader for Nostr WASM cassettes",
    long_description=long_description,
    long_description_content_type="text/markdown",
    url="https://github.com/yourusername/cassette",
    packages=find_packages(),
    classifiers=[
        "Programming Language :: Python :: 3",
        "License :: OSI Approved :: MIT License",
        "Operating System :: OS Independent",
    ],
    python_requires=">=3.8",
    install_requires=[
        "wasmtime>=24.0.0",
    ],
)