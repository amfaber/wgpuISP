import matplotlib.pyplot as plt
import numpy as np
from pathlib import Path
here = Path(__file__).parent
__all__ = ["np", "plt"]

input = np.fromfile(here/"input.bin", dtype = "float32")
__all__.append("input")
output = np.fromfile(here/"output.bin", dtype = "float32")
__all__.append("output")
