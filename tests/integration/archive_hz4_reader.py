"""Narrow HZ4 oracle bookkeeping; NOT a product parser or output authority.
Only the parent-approved fixed four-entry fixture is covered. Everything else fails closed.
"""
import hashlib
import lzma
import struct
import zlib

SEVEN_SHA = '4BCC005758C5E604C0AAA257B6097846A4A708E62A9C0C7471FA360FAD863F1F'
TAR_SHA = '92FAFED485AF0FE4C291457DC3C00D0241F6A136731BAB63EF18A3F0C949D37A'
LIMIT = 1024 * 1024
# Full decoded metadata, including exact stream map, CRCs, names, empty-stream bitmap,
# original timestamps and canonical TTTF WinAttributes. New directory metadata is undefined.
WITNESSED_HEADER = bytes.fromhex(
    '01040600020918190a01ee329d6ad516f98500070b020001212101100121210110'
    '0c141500080a0125a984f4b9cefa05000005040e0130114b006600690072007300'
    '74002e0074007800740000007300650063006f006e0064002e0074007800740000'
    '0065006d0070007400790000007000750062006c00690073006800650064000000'
    '141b00e00000006dc64717da0100006dc64717da0100006dc64717da01'
    '150f00e0002000000020000000100000000000')
PAYLOADS = (b'first owned payload\n', b'second owned payload\n')


class Cursor:
    def __init__(self, data):
        self.data = data
        self.at = 0

    def take(self, count):
        assert 0 <= count <= len(self.data) - self.at, 'bounded descriptor'
        value = self.data[self.at:self.at + count]
        self.at += count
        return value

    def byte(self):
        return self.take(1)[0]

    def expect(self, value):
        assert self.take(len(value)) == value, 'unsupported descriptor structure'

    def number(self):
        first = self.byte()
        mask = 0x80
        value = 0
        for index in range(8):
            if not first & mask:
                return value | ((first & (mask - 1)) << (8 * index))
            value |= self.byte() << (8 * index)
            mask >>= 1
        return value


def checked_header(data):
    assert 32 < len(data) <= LIMIT and data[:8] == b'7z\xbc\xaf\x27\x1c\x00\x04', 'signature/size'
    assert zlib.crc32(data[12:32]) == struct.unpack_from('<I', data, 8)[0], 'start CRC'
    offset, length, crc = struct.unpack_from('<QQI', data, 12)
    assert 0 < length <= LIMIT and 32 + offset + length == len(data), 'next bounds/tail'
    header = data[32 + offset:]
    assert zlib.crc32(header) == crc, 'next CRC'
    if header[0] == 1:
        return header, data[32:32 + offset], {'encoding': 'plain', 'next_crc': crc}
    c = Cursor(header)
    c.expect(b'\x17\x06')
    pack_pos = c.number()
    assert c.number() == 1
    c.expect(b'\x09')
    packed_size = c.number()
    c.expect(b'\x0a\x01')
    packed_crc = struct.unpack('<I', c.take(4))[0]
    c.expect(b'\x00\x07\x0b\x01\x00\x01\x23\x03\x01\x01\x05')
    props = c.take(5)
    assert props[0] < 225
    dictionary = int.from_bytes(props[1:], 'little')
    assert 4096 <= dictionary <= 8 * LIMIT, 'decoder memory bound'
    c.expect(b'\x0c')
    unpacked_size = c.number()
    assert 0 < unpacked_size <= LIMIT
    c.expect(b'\x00\x08\x0a\x01')
    decoded_crc = struct.unpack('<I', c.take(4))[0]
    c.expect(b'\x00\x00')
    assert c.at == len(header) and pack_pos + packed_size == offset, 'complete descriptor/pack bounds'
    packed = data[32 + pack_pos:32 + offset]
    assert zlib.crc32(packed) == packed_crc, 'encoded pack CRC'
    prop = props[0]
    filters = [{'id': lzma.FILTER_LZMA1, 'dict_size': dictionary,
                'lc': prop % 9, 'lp': (prop // 9) % 5, 'pb': prop // 45}]
    decoder = lzma.LZMADecompressor(format=lzma.FORMAT_RAW, filters=filters)
    raw = decoder.decompress(packed, max_length=unpacked_size + 1)
    assert len(raw) == unpacked_size and not decoder.unused_data, 'bounded decoded length'
    assert zlib.crc32(raw) == decoded_crc, 'decoded CRC'
    return raw, data[32:32 + pack_pos], {'encoding': 'LZMA1', 'next_crc': crc,
            'packed_crc': packed_crc, 'decoded_crc': decoded_crc, 'decoded_length': unpacked_size}


def qualify_bytes(data, seven_sha, tar_sha, tar_version):
    assert seven_sha == SEVEN_SHA and tar_sha == TAR_SHA, 'unapproved reader identity'
    assert tar_version.startswith('bsdtar 3.7.7 - libarchive 3.7.7 '), 'unapproved reader version'
    raw, body, facts = checked_header(data)
    assert raw == WITNESSED_HEADER, 'not the approved full four-entry metadata structure'
    assert len(body) == 49, 'exact main pack extent'
    for packed, crc, expected in zip((body[:24], body[24:]), (0x6A9D32EE, 0x85F916D5), PAYLOADS):
        assert zlib.crc32(packed) == crc, 'main packed CRC'
        decoder = lzma.LZMADecompressor(format=lzma.FORMAT_RAW,
                                      filters=[{'id': lzma.FILTER_LZMA2, 'dict_size': LIMIT}])
        actual = decoder.decompress(packed, max_length=len(expected) + 1)
        assert actual == expected and decoder.eof and not decoder.unused_data, 'exact main stream/EOF'
    return dict(facts, status='reader-unsupported', reason='HZ4-7Z02-pinned-bsdtar-partial-inline-WinAttributes',
                entries=4, defined=[True, True, True, False], old_attributes=[32, 32, 16],
                new_directory_attributes=None, decoded_header_sha256=hashlib.sha256(raw).hexdigest().upper())
