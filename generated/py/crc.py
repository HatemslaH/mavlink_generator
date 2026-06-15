class CrcX25:
    _X25_INIT_CRC = 0xFFFF

    def __init__(self) -> None:
        self._crc = self._X25_INIT_CRC

    @property
    def crc(self) -> int:
        return self._crc & 0xFFFF

    def accumulate(self, byte: int) -> None:
        byte = byte & 0xFF
        tmp = byte ^ (self._crc & 0xFF)
        tmp &= 0xFF
        tmp ^= (tmp << 4) & 0xFF
        self._crc = (
            (self._crc >> 8)
            ^ ((tmp << 8) & 0xFFFF)
            ^ ((tmp << 3) & 0xFFFF)
            ^ (tmp >> 4)
        )

    def accumulate_string(self, text: str) -> None:
        for code_unit in text.encode("ascii", errors="ignore"):
            self.accumulate(code_unit)
