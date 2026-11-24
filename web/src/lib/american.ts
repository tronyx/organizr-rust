// PHP Organizr writes a few words in British spelling. The server passes PHP's texts on
// as they are (the API keeps PHP's answers), and the interface shows them in American
// spelling. Only whole words change, so names such as `netdata1Colour` stay as they are.

/** The British words of PHP's texts, each with its American spelling. */
const AMERICAN: Record<string, string> = {
  Colour: 'Color',
  colour: 'color',
  Colours: 'Colors',
  colours: 'colors',
  Cancelled: 'Canceled',
  cancelled: 'canceled',
};

const BRITISH_WORD = new RegExp(`\\b(${Object.keys(AMERICAN).join('|')})\\b`, 'g');

/** A text of PHP as the interface shows it: in American spelling. */
export function american(text: string): string;
export function american(text: string | undefined): string | undefined;
export function american(text: string | undefined): string | undefined {
  return text?.replace(BRITISH_WORD, (word) => AMERICAN[word] ?? word);
}
