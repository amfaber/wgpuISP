import matplotlib.pyplot as plt
import numpy as np
from pathlib import Path
here = Path(__file__).parent
__all__ = ["np", "plt"]

input = np.fromfile(here/"input.bin", dtype = "float32")
__all__.append("input")
black_level = np.fromfile(here/"black_level.bin", dtype = "float32")
__all__.append("black_level")
temp_mean = np.fromfile(here/"temp_mean.bin", dtype = "float32")
__all__.append("temp_mean")
mean = np.fromfile(here/"mean.bin", dtype = "float32")
__all__.append("mean")
output = np.fromfile(here/"output.bin", dtype = "float32")
__all__.append("output")
