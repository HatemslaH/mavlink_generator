using System.IO.Ports;

namespace MavlinkSitlGcs;

/// <summary>Lists available serial ports and reads a selection from stdin.</summary>
public static class PortPicker
{
    public static string PickSerialPort()
    {
        var portNames = SerialPort.GetPortNames();
        if (portNames.Length == 0)
        {
            throw new InvalidOperationException(
                "No serial ports found. Connect SITL or a USB adapter.");
        }

        Array.Sort(portNames);

        Console.WriteLine();
        Console.WriteLine("Available serial ports:");
        for (var index = 0; index < portNames.Length; index++)
        {
            Console.WriteLine($"  [{index}] {portNames[index]}");
        }

        Console.WriteLine();
        Console.Write($"Select port [0-{portNames.Length - 1}]: ");

        var line = Console.ReadLine()?.Trim();
        if (string.IsNullOrEmpty(line))
        {
            throw new InvalidOperationException("Port selection required");
        }

        if (!int.TryParse(line, out var selected) || selected < 0 || selected >= portNames.Length)
        {
            throw new InvalidOperationException($"Invalid port selection: {line}");
        }

        var portName = portNames[selected];
        Console.WriteLine($"Selected {portName}");
        return portName;
    }

    /// <summary>Parse <c>--baud &lt;rate&gt;</c> from CLI arguments (default 57600).</summary>
    public static int ParseBaudRate(string[] args, int defaultBaud = 57600)
    {
        for (var index = 0; index < args.Length - 1; index++)
        {
            if (args[index] == "--baud")
            {
                if (!int.TryParse(args[index + 1], out var value) || value <= 0)
                {
                    throw new ArgumentException($"Invalid --baud value: {args[index + 1]}");
                }

                return value;
            }
        }

        return defaultBaud;
    }
}
