import json
import time
from pathlib import Path
from typing import Optional
from .types import VerifyResult, PipelineStage

_WASM_PATH = Path(__file__).parent / "ordyn_wasm.wasm"

def verify_via_wasm(source: str) -> Optional[VerifyResult]:
    """Attempts to execute the bundled WebAssembly core using wasmtime or wasmer if available."""
    try:
        import wasmtime # type: ignore
        engine = wasmtime.Engine()
        store = wasmtime.Store(engine)
        module = wasmtime.Module.from_file(engine, str(_WASM_PATH))
        instance = wasmtime.Instance(store, module, [])
        exports = instance.exports(store)

        t0 = time.perf_counter()
        encoded = source.encode("utf-8")
        alloc_fn = exports["ordyn_alloc"]
        verify_fn = exports["ordyn_verify_source"]
        memory = exports["memory"]

        ptr = alloc_fn(store, len(encoded))
        data_ptr = memory.data_ptr(store)
        
        # Write bytes into memory
        import ctypes
        ctypes.memmove(data_ptr + ptr, encoded, len(encoded))

        # Call verification
        res_ptr = verify_fn(store, ptr, len(encoded))

        # Read null-terminated JSON string
        mem_slice = memory.data_ptr(store)
        curr = res_ptr
        chars = []
        while True:
            b = ctypes.c_uint8.from_address(mem_slice + curr).value
            if b == 0:
                break
            chars.append(b)
            curr += 1

        json_str = bytes(chars).decode("utf-8")
        data = json.loads(json_str)
        elapsed_us = max(10, int((time.perf_counter() - t0) * 1_000_000))

        stages = [
            PipelineStage(s["name"], s["status"], s["details"])
            for s in data.get("stages", [])
        ]

        return VerifyResult(
            success=data.get("success", False),
            total_declarations=data.get("total_declarations", 0),
            entities=data.get("entities", []),
            capabilities=data.get("capabilities", []),
            boundaries=data.get("boundaries", []),
            contracts_count=data.get("contracts_count", 0),
            tir_functions=data.get("tir_functions", 0),
            stages=stages,
            error_message=data.get("error_message"),
            tir_summary=data.get("tir_summary"),
            elapsed_us=elapsed_us,
        )
    except ImportError:
        return None
