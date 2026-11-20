from dataclasses import dataclass, field
from typing import List, Optional

@dataclass
class PipelineStage:
    name: str
    status: str
    details: str

@dataclass
class VerifyResult:
    success: bool
    total_declarations: int = 0
    entities: List[str] = field(default_factory=list)
    capabilities: List[str] = field(default_factory=list)
    boundaries: List[str] = field(default_factory=list)
    contracts_count: int = 0
    tir_functions: int = 0
    stages: List[PipelineStage] = field(default_factory=list)
    error_message: Optional[str] = None
    tir_summary: Optional[str] = None
    elapsed_us: int = 0

    @property
    def is_valid(self) -> bool:
        return self.success

@dataclass
class RunResult:
    success: bool
    trapped: bool
    output: Optional[str] = None
    fuel_consumed: int = 0
    fuel_remaining: int = 0
    stages: List[PipelineStage] = field(default_factory=list)
    error_message: Optional[str] = None
    elapsed_us: int = 0

    @property
    def is_valid(self) -> bool:
        return self.success and not self.trapped
