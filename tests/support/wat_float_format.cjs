// テストhost用の既存9/17桁規則です。二進値を整数比として丸めます。
// JSのtoPrecisionとは異なり、丁度中間の値は偶数へ丸めます。
function format(value, digits) {
  if (Number.isNaN(value)) return 'NaN';
  if (!Number.isFinite(value)) return value < 0 ? '-inf' : 'inf';
  const negative = value < 0 || Object.is(value, -0);
  const sign = negative ? '-' : '';
  value = Math.abs(value);
  if (value === 0) return sign + '0';
  const buffer = new ArrayBuffer(8);
  const view = new DataView(buffer);
  view.setFloat64(0, value, false);
  const bits = view.getBigUint64(0, false);
  const exponent = Number((bits >> 52n) & 2047n);
  const mantissa = (bits & ((1n << 52n) - 1n)) | (exponent ? 1n << 52n : 0n);
  const power = (exponent || 1) - 1023 - 52;
  let numerator = mantissa, denominator = 1n;
  if (power >= 0) numerator <<= BigInt(power);
  else denominator <<= BigInt(-power);
  const ten = n => 10n ** BigInt(n);
  let decimalExponent = Math.floor(Math.log10(value));
  const belowPower = e => e >= 0 ? numerator < denominator * ten(e) : numerator * ten(-e) < denominator;
  while (belowPower(decimalExponent)) --decimalExponent;
  while (!belowPower(decimalExponent + 1)) ++decimalExponent;
  const scale = digits - 1 - decimalExponent;
  if (scale >= 0) numerator *= ten(scale);
  else denominator *= ten(-scale);
  let rounded = numerator / denominator;
  const remainder = numerator % denominator;
  if (remainder * 2n > denominator || (remainder * 2n === denominator && rounded % 2n)) ++rounded;
  if (rounded === ten(digits)) { rounded /= 10n; ++decimalExponent; }
  const text = rounded.toString().padStart(digits, '0');
  const trim = s => s.includes('.') ? s.replace(/0+$/, '').replace(/\.$/, '') : s;
  if (decimalExponent < -4 || decimalExponent >= digits) {
    return sign + trim(text[0] + '.' + text.slice(1)) + 'e' +
      (decimalExponent < 0 ? '-' : '+') + Math.abs(decimalExponent).toString().padStart(2, '0');
  }
  const point = decimalExponent + 1;
  const decimal = point <= 0 ? '0.' + '0'.repeat(-point) + text :
    point >= text.length ? text + '0'.repeat(point - text.length) : text.slice(0, point) + '.' + text.slice(point);
  return sign + trim(decimal);
}
function chunk(value, digits, offset) {
  const text = Buffer.from(format(value, digits), 'ascii');
  if (!Number.isInteger(offset) || offset < 0 || offset % 8 || offset >= text.length) throw new Error('invalid format offset');
  const part = Buffer.alloc(8);
  text.copy(part, 0, offset, offset + 8);
  return [part.readBigInt64LE(), text.length];
}
module.exports = { format, chunk };
