/**
 * Synthesised alarm signals (Web Audio). No audio files, no network.
 * The island window needs autoplay without user gesture (see BACKEND.md).
 */

/** Available signal sounds. */
export type SignalSound = 'chime' | 'pulse' | 'digital';

let ctx: AudioContext | null = null;

function audio(): AudioContext | null {
  if (typeof AudioContext === 'undefined') {
    return null;
  }
  ctx ??= new AudioContext();
  if (ctx.state === 'suspended') {
    void ctx.resume();
  }
  return ctx;
}

function tone(ac: AudioContext, at: number, freq: number, length: number, volume: number, type: OscillatorType): void {
  const osc = ac.createOscillator();
  const gain = ac.createGain();
  osc.type = type;
  osc.frequency.value = freq;
  gain.gain.setValueAtTime(0, at);
  gain.gain.linearRampToValueAtTime(volume, at + 0.012);
  gain.gain.exponentialRampToValueAtTime(0.0001, at + length);
  osc.connect(gain).connect(ac.destination);
  osc.start(at);
  osc.stop(at + length + 0.05);
}

/**
 * Plays one phrase of a signal.
 *
 * @param sound - Which signal.
 * @param volume - 0…1.
 */
export function playSignal(sound: SignalSound, volume: number): void {
  const ac = audio();
  if (!ac) {
    return;
  }
  const t = ac.currentTime + 0.02;
  const v = Math.max(0.0001, Math.min(1, volume)) * 0.35;
  if (sound === 'chime') {
    tone(ac, t, 880, 1.2, v, 'sine');
    tone(ac, t, 1320, 0.9, v * 0.4, 'sine');
    tone(ac, t + 0.32, 1174.7, 1.4, v, 'sine');
    tone(ac, t + 0.32, 1760, 1.0, v * 0.35, 'sine');
  } else if (sound === 'pulse') {
    for (let i = 0; i < 3; i++) {
      tone(ac, t + i * 0.22, 660, 0.18, v, 'triangle');
    }
  } else {
    for (let i = 0; i < 4; i++) {
      tone(ac, t + i * 0.14, 2093, 0.08, v * 0.6, 'square');
    }
  }
}

/**
 * Repeats a signal until stopped (max. one minute).
 *
 * @param sound - Which signal.
 * @param volume - 0…1.
 * @returns Stop function.
 */
export function loopSignal(sound: SignalSound, volume: number): () => void {
  playSignal(sound, volume);
  const started = Date.now();
  const timer = setInterval(() => {
    if (Date.now() - started > 60_000) {
      clearInterval(timer);
      return;
    }
    playSignal(sound, volume);
  }, 2200);
  return () => clearInterval(timer);
}
