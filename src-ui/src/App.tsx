import { SpotlightWindow } from './components/layout/SpotlightWindow';

export default function App() {
  return (
    <div className="h-screen w-screen flex items-start justify-center pt-[15vh] px-4">
      <div className="w-full max-w-[560px]">
        <SpotlightWindow />
      </div>
    </div>
  );
}
