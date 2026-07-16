"""Content-addressed ML model registry with version pinning."""

from __future__ import annotations

import hashlib
import json
import pickle  # noqa: S403
import re
import tempfile
from dataclasses import dataclass
from datetime import UTC, datetime
from pathlib import Path
from typing import Any, Protocol

from pydantic import BaseModel, ConfigDict, Field

_INDEX_FILE = "registry.json"
_SAFE_PART = re.compile(r"^[A-Za-z0-9_.-]+$")
_SEMVER = re.compile(r"^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:[-+][0-9A-Za-z.-]+)?$")


class ModelRegistryError(ValueError):
    """Raised when model registry operations fail."""


class ModelEntry(BaseModel):
    """Metadata for one registered model version."""

    model_config = ConfigDict(strict=True, extra="forbid")

    name: str
    version: str
    sha256: str
    format: str
    artifact_path: str
    metadata: dict[str, Any] = Field(default_factory=dict)
    created_at: datetime


class _RegistryIndex(BaseModel):
    model_config = ConfigDict(strict=True, extra="forbid")

    models: dict[str, dict[str, ModelEntry]] = Field(default_factory=dict)
    pins: dict[str, dict[str, str]] = Field(default_factory=dict)


class ModelAdapter(Protocol):
    """Serialization adapter for model artifacts."""

    format: str
    extension: str

    def dump(self, model: Any, path: Path) -> None: ...

    def load(self, path: Path) -> Any: ...


@dataclass(frozen=True)
class PickleAdapter:
    """Fallback pickle adapter for plain Python test models."""

    format: str = "pickle"
    extension: str = ".pkl"

    def dump(self, model: Any, path: Path) -> None:
        with path.open("wb") as handle:
            pickle.dump(model, handle, protocol=pickle.HIGHEST_PROTOCOL)

    def load(self, path: Path) -> Any:
        with path.open("rb") as handle:
            return pickle.load(handle)  # noqa: S301


@dataclass(frozen=True)
class SklearnJoblibAdapter:
    """scikit-learn/joblib model adapter."""

    format: str = "sklearn"
    extension: str = ".joblib"

    def dump(self, model: Any, path: Path) -> None:
        import joblib

        joblib.dump(model, path)

    def load(self, path: Path) -> Any:
        import joblib

        return joblib.load(path)


@dataclass(frozen=True)
class PyTorchAdapter:
    """PyTorch .pt adapter."""

    format: str = "pytorch"
    extension: str = ".pt"

    def dump(self, model: Any, path: Path) -> None:
        import torch

        torch.save(model, path)

    def load(self, path: Path) -> Any:
        import torch

        return torch.load(path, map_location="cpu", weights_only=False)


@dataclass(frozen=True)
class OnnxAdapter:
    """ONNX binary artifact adapter."""

    format: str = "onnx"
    extension: str = ".onnx"

    def dump(self, model: Any, path: Path) -> None:
        if isinstance(model, bytes):
            path.write_bytes(model)
            return
        if hasattr(model, "SerializeToString"):
            path.write_bytes(model.SerializeToString())
            return
        raise ModelRegistryError("onnx adapter expects bytes or a serializable ONNX model")

    def load(self, path: Path) -> bytes:
        return path.read_bytes()


class ModelRegistry:
    """Save, load, and list content-addressed model artifacts."""

    def __init__(self, root: str | Path) -> None:
        self.root = Path(root)
        self.models_root = self.root / "models"
        Path(self.models_root).mkdir(exist_ok=True, parents=True)
        self.index_path = self.root / _INDEX_FILE
        self.adapters: dict[str, ModelAdapter] = {
            "pickle": PickleAdapter(),
            "sklearn": SklearnJoblibAdapter(),
            "joblib": SklearnJoblibAdapter(),
            "pytorch": PyTorchAdapter(),
            "torch": PyTorchAdapter(),
            "onnx": OnnxAdapter(),
        }

    def save(
        self,
        name: str,
        version: str,
        model: Any,
        metadata: dict[str, Any] | None = None,
        *,
        format: str | None = None,  # noqa: A002
        pin: bool = True,
        overwrite: bool = False,
    ) -> ModelEntry:
        """Save a model under models/{name}/{version}/blobs/{sha}.{ext}."""
        self._validate_name(name)
        self._validate_version(version)
        adapter = self._adapter_for(format or self._infer_format(model))
        payload = self._serialize(adapter, model)
        sha256 = hashlib.sha256(payload).hexdigest()

        index = self._read_index()
        versions = index.models.setdefault(name, {})
        existing = versions.get(version)
        if existing and not overwrite:
            raise ModelRegistryError(f"model {name!r} version {version!r} already exists")

        version_dir = self.models_root / name / version
        blob_dir = version_dir / "blobs"
        Path(blob_dir).mkdir(exist_ok=True, parents=True)
        artifact_path = blob_dir / f"{sha256}{adapter.extension}"
        if not artifact_path.exists():
            artifact_path.write_bytes(payload)

        entry = ModelEntry(
            name=name,
            version=version,
            sha256=sha256,
            format=adapter.format,
            artifact_path=str(artifact_path.relative_to(self.root)),
            metadata=metadata or {},
            created_at=datetime.now(UTC),
        )
        versions[version] = entry
        if pin:
            index.pins[name] = {"version": version, "sha256": sha256}
        self._write_index(index)
        return entry

    def load(self, name: str, version: str | None = None) -> Any:
        """Load a model by explicit version or the pinned version."""
        entry = self.get(name, version)
        return self._adapter_for(entry.format).load(self.root / entry.artifact_path)

    def list(self, name: str | None = None) -> list[ModelEntry]:
        """List registered model versions newest first."""
        index = self._read_index()
        selected = {name: index.models.get(name, {})} if name else index.models
        entries = [entry for versions in selected.values() for entry in versions.values()]
        return sorted(entries, key=lambda entry: entry.created_at, reverse=True)

    def get(self, name: str, version: str | None = None) -> ModelEntry:
        """Return metadata for a model version, validating pinned SHA when used."""
        index = self._read_index()
        versions = index.models.get(name)
        if not versions:
            raise ModelRegistryError(f"model {name!r} is not registered")

        pin = index.pins.get(name)
        resolved = version or (pin or {}).get("version")
        if not resolved:
            resolved = max(versions.values(), key=lambda entry: entry.created_at).version
        entry = versions.get(resolved)
        if entry is None:
            raise ModelRegistryError(f"model {name!r} version {resolved!r} is not registered")
        if version is None and pin and entry.sha256 != pin.get("sha256"):
            raise ModelRegistryError(f"pinned SHA mismatch for model {name!r} version {resolved!r}")
        return entry

    def pin(self, name: str, version: str) -> ModelEntry:
        """Pin a model to an exact semver and SHA256 artifact digest."""
        index = self._read_index()
        entry = self.get(name, version)
        index.pins[name] = {"version": entry.version, "sha256": entry.sha256}
        self._write_index(index)
        return entry

    def pinned_version(self, name: str) -> str | None:
        """Return the pinned semver for a model, if present."""
        pin = self._read_index().pins.get(name)
        return pin.get("version") if pin else None

    def _serialize(self, adapter: ModelAdapter, model: Any) -> bytes:
        with tempfile.TemporaryDirectory() as temp_dir:
            path = Path(temp_dir) / f"model{adapter.extension}"
            adapter.dump(model, path)
            return path.read_bytes()

    def _read_index(self) -> _RegistryIndex:
        if not self.index_path.exists():
            return _RegistryIndex()
        return _RegistryIndex.model_validate_json(self.index_path.read_text())

    def _write_index(self, index: _RegistryIndex) -> None:
        payload = index.model_dump(mode="json")
        self.index_path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n")

    def _adapter_for(self, format_name: str) -> ModelAdapter:
        adapter = self.adapters.get(format_name.lower())
        if adapter is None:
            raise ModelRegistryError(f"unsupported model format: {format_name!r}")
        return adapter

    @staticmethod
    def _infer_format(model: Any) -> str:
        module = type(model).__module__.split(".", maxsplit=1)[0]
        if module == "torch":
            return "pytorch"
        if module == "sklearn":
            return "sklearn"
        return "pickle"

    @staticmethod
    def _validate_name(value: str) -> None:
        if not value or "/" in value or not _SAFE_PART.fullmatch(value):
            raise ModelRegistryError(f"invalid model name: {value!r}")

    @staticmethod
    def _validate_version(value: str) -> None:
        if (
            not value
            or "/" in value
            or not _SAFE_PART.fullmatch(value)
            or not _SEMVER.fullmatch(value)
        ):
            raise ModelRegistryError(f"version must be semver: {value!r}")
