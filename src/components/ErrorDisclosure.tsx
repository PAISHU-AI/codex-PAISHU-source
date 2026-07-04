import { ChevronDown } from "lucide-react";

interface ErrorDisclosureProps {
  title: string;
  messages: string[];
}

export function ErrorDisclosure({ title, messages }: ErrorDisclosureProps) {
  if (messages.length === 0) return null;
  return (
    <details className="error-disclosure">
      <summary>
        <ChevronDown size={14} />
        {title}
        <span>{messages.length}</span>
      </summary>
      <ul>
        {messages.map((message, index) => (
          <li key={`${message}-${index}`}>{message}</li>
        ))}
      </ul>
    </details>
  );
}
